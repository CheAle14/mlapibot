use linkify::LinkKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstrAttempt<'md> {
    markdown: &'md str,
    /// If present, the position of the item that forced `markdown` to be larger
    /// than the requested `max`.
    start_of_token: Option<usize>,
}

impl<'md> SubstrAttempt<'md> {
    fn new(markdown: &'md str) -> Self {
        Self {
            markdown,
            start_of_token: None,
        }
    }

    fn token(markdown: &'md str, start_of_token: usize) -> Self {
        Self {
            markdown,
            start_of_token: Some(start_of_token),
        }
    }

    pub fn text(&self) -> &'md str {
        self.markdown
    }

    pub fn len(&self) -> usize {
        self.markdown.len()
    }

    pub fn removalable_len(&self) -> Option<usize> {
        self.start_of_token.map(|idx| self.len() - idx)
    }

    pub fn trim_to_token(&self) -> Option<Self> {
        if let Some(token) = self.start_of_token {
            Some(substr_markdown(self.markdown, token))
        } else {
            None
        }
    }
}

fn get_repeated_prefix<'md>(text: &'md str) -> (&'md str, &'md str) {
    let mut iter = text.char_indices();
    let Some((_, looking_for)) = iter.next() else {
        return ("", "");
    };

    for (idx, chr) in iter {
        if chr != looking_for {
            return text.split_at(idx);
        }
    }

    // the text contains entirely one character repeated (or just one character)
    (text, "")
}

fn is_valid_subreddit_chr(chr: char) -> bool {
    // technically the first character cannot be _, but ¯\_(ツ)_/¯
    chr.is_ascii_alphanumeric() || chr == '_'
}

fn is_valid_username_chr(chr: char) -> bool {
    chr.is_ascii_alphanumeric()
}

/// Attempts to limit `markdown` to be at most `max` characters, but it may be larger
/// if doing so would break markdown format. In that case, the text returned is extended
/// to the end of that particular syntax (e.g. **bold text**).
pub fn substr_markdown<'md>(markdown: &'md str, max: usize) -> SubstrAttempt<'md> {
    if max == 0 {
        return SubstrAttempt {
            markdown: "",
            start_of_token: None,
        };
    }

    if markdown.len() < max {
        return SubstrAttempt {
            markdown,
            start_of_token: None,
        };
    };

    let mut link_finder = linkify::LinkFinder::new();
    link_finder.kinds(&[LinkKind::Url]);

    let mut current_idx = 0;

    while current_idx < markdown.len() {
        let start = current_idx;
        if current_idx >= max {
            return SubstrAttempt {
                markdown: &markdown[..current_idx],
                start_of_token: None,
            };
        }

        let remaining = &markdown[current_idx..];

        match remaining.chars().next() {
            Some('_' | '*') => {
                let (underscores, remaining) = get_repeated_prefix(remaining);
                current_idx += underscores.len();

                let Some(closing) = remaining.find(underscores) else {
                    continue;
                };

                current_idx += closing + underscores.len();
            }
            Some('~') if remaining.starts_with("~~") => {
                const LEN: usize = "~~".len();

                current_idx += LEN;
                let Some(closing) = (&remaining[LEN..]).find("~~") else {
                    continue;
                };
                current_idx += closing + LEN;
            }
            Some('<') => {
                current_idx += 1;
                let Some(closing) = (&remaining[1..]).find('>') else {
                    continue;
                };
                current_idx += closing + 1;
            }
            Some('>') if remaining.starts_with(">!") => {
                const LEN: usize = ">!".len();

                current_idx += LEN;
                let Some(closing) = (&remaining[LEN..]).find("!<") else {
                    continue;
                };
                current_idx += closing + LEN;
            }
            Some('^') if !remaining.starts_with("^(") => {
                current_idx += 1;

                let Some(closing) = (&remaining[1..]).find(&[' ', '\t', '\n']) else {
                    continue;
                };

                current_idx += closing;
            }
            Some('^') /* remaining.starts_with("^)") */ => {
                current_idx += 2;

                let Some(closing) = (&remaining[2..]).find(')') else {
                    continue;
                };

                current_idx += closing + 1;
            }
            Some('`') => {
                current_idx += 1;

                let Some(closing) = (&remaining[1..]).find('`') else {
                    continue;
                };

                current_idx += closing + 1;
            }
            Some('u') if remaining.starts_with("u/") => {
                current_idx += 2;

                let remaining = &remaining[2..];
                let closing = remaining.find(|c| !is_valid_username_chr(c)).unwrap_or(remaining.len());

                current_idx += closing;
            }
            Some('r') if remaining.starts_with("r/") => {
                current_idx += 2;

                let remaining = &remaining[2..];
                let closing = remaining.find(|c| !is_valid_subreddit_chr(c)).unwrap_or(remaining.len());

                current_idx += closing;
            }
            Some('[') => {
                current_idx += 1;

                let remaining = &remaining[1..];
                let Some(midpart) = remaining.find("](") else {
                    continue;
                };

                current_idx += midpart + 2;

                let remaining = &remaining[(midpart + 2)..];
                let Some(closing) = remaining.find(')') else {
                    continue;
                };

                current_idx += closing + 1;
            }
            Some('h') if remaining.starts_with("http") => {
                let Some(link) = link_finder.links(remaining).next() else {
                    current_idx += 1;
                    continue;
                };

                current_idx += link.end();
            }

            Some(c) => {
                current_idx += c.len_utf8();
                continue;
            },
            None => break,
        };

        if current_idx >= max {
            return SubstrAttempt::token(&markdown[..current_idx], start);
        }
    }

    SubstrAttempt {
        markdown,
        start_of_token: None,
    }
}

#[derive(Debug)]
struct SubstrInput<'a> {
    order: usize,
    original: &'a str,
    allowance: usize,
    attempt: SubstrAttempt<'a>,
}

/// Attempts to shorten each of the items in the iterator until their total length is below max.
///
/// The returned vec contains each item provided by the `items` iterator in the same order.
pub fn substr_markdown_many<'md>(
    items: impl Iterator<Item = &'md str>,
    max: usize,
) -> Vec<SubstrAttempt<'md>> {
    // The basic algorithm is to evenly divide the `max` across all items,
    // which this calls an "allowance".
    //
    // For every item that is already smaller than the max, the unneeded
    // difference is taken from that item's allowance and pooled.
    //
    // Starting at the shortest item that is larger than max, the extra
    // pooled allowance is then used to attempt to extend that item completely.
    // Any leftover is then rolled over to the next shortest item.
    //
    // After all allowance has been allocated, if the total sum length is still over
    // the max then items which can have a markdown element removed (e.g. `hello *world*`, the `*world*` part)
    // are considered. The item whose removable length (e.g. len("*world*") == 7) is closest
    // to the amount needed to bring the current sum below max is truncated accordingly.
    // This repeats until the current sum is below max, or there are no more able to be removed.
    //
    // If the sum is still not below max, the amount allocated to each item is reduced by one
    // and the process repeats again. This should always succeed at length zero, since that is an
    // empty string.
    let mut substrs: Vec<SubstrInput> = items
        .enumerate()
        .map(|(order, markdown)| SubstrInput {
            order,
            original: markdown,
            allowance: usize::MAX,
            attempt: SubstrAttempt::new(markdown),
        })
        .collect();

    substrs.sort_unstable_by_key(|a| a.original.len());

    'item_max: for items_max in (0..=(max / substrs.len())).rev() {
        let total_items_max = items_max * substrs.len();

        let remainder = max - total_items_max;

        let point = substrs.partition_point(|a| a.original.len() <= items_max);
        let (keep_as_is, need_to_reduce) = unsafe { substrs.split_at_mut_unchecked(point) };

        let mut extra_allowance = remainder;

        for item in keep_as_is {
            item.allowance = item.original.len();
            item.attempt = SubstrAttempt::new(item.original);
            extra_allowance += items_max - item.original.len();
        }

        for too_big in need_to_reduce {
            too_big.attempt = substr_markdown(too_big.original, items_max + extra_allowance);
            too_big.allowance = (items_max + extra_allowance).min(too_big.attempt.len());

            if let Some(allowance_used) = too_big.allowance.checked_sub(items_max) {
                extra_allowance -= allowance_used;
            }
        }

        let mut current_sum = substrs.iter().map(|a| a.attempt.len()).sum::<usize>();

        while current_sum > max {
            let global_distance = current_sum - max;

            let mut closest_to_distance: Option<(usize, &mut SubstrAttempt)> = None;
            for substr in &mut substrs {
                let Some(removable) = substr.attempt.removalable_len() else {
                    continue;
                };

                let distance_to_global = removable.abs_diff(global_distance);

                match closest_to_distance {
                    Some((r, _)) => {
                        if distance_to_global < r {
                            closest_to_distance = Some((distance_to_global, &mut substr.attempt));
                        }
                    }
                    None => closest_to_distance = Some((distance_to_global, &mut substr.attempt)),
                }
            }

            match closest_to_distance {
                Some((_, substr)) => {
                    let new = substr.trim_to_token().expect("removable not None");
                    let diff = substr.len() - new.len();
                    current_sum -= diff;
                    *substr = new;
                }
                None => {
                    continue 'item_max;
                }
            }
        }

        substrs.sort_unstable_by_key(|a| a.order);
        return substrs.into_iter().map(|a| a.attempt).collect();
    }

    panic!("unable to perform *any* reduction?? even empty strings should satisfy!");
}

/// Allows constructing a plan to layout multiple markdown-formatted strings
/// that may need to be cut down in size.
///
/// # Example
///
/// ```
/// use std::fmt::Write;
/// use mlapibot_markdown::substr::LayoutPlan;
///
/// # fn main() -> std::fmt::Result {
///
/// let strings = vec!["Some **markdown** text", "Another _text_ here"];
/// let mut plan = LayoutPlan::new();
/// // Add prefix to template
/// writeln!(plan, "Hello world")?;
///
/// for md in &strings {
///     plan.argument(|arg| {
///         // Each argument can have its own templated text too
///         write!(arg, "> ")?;
///         // Note where the argument's size-varying text should be
///         arg.placeholder();
///         write!(arg, "\n")
///     })?;
/// }
///
/// writeln!(plan, "\n---\nA suffix here")?;
///
/// // Finally, inserts the items into the template at their placeholder positions
/// assert_eq!(plan.execute(strings), r#"Hello world
/// > Some **markdown** text
/// > Another _text_ here
///
/// ---
/// A suffix here
/// "#);
///
/// // If the text is too large, we could shorten some of the strings and retry.
///
/// # Ok(())
/// # }
/// ```
///
pub struct LayoutPlan {
    template: String,
    args: Vec<usize>,
}

pub struct LayoutArgPlan<'plan> {
    plan: &'plan mut LayoutPlan,
}

impl LayoutPlan {
    pub fn new() -> Self {
        Self {
            template: String::new(),
            args: Vec::new(),
        }
    }

    pub fn argument<F>(&mut self, callback: F) -> std::fmt::Result
    where
        F: FnOnce(&mut LayoutArgPlan<'_>) -> std::fmt::Result,
    {
        let mut arg = LayoutArgPlan { plan: self };
        callback(&mut arg)
    }

    pub fn len(&self) -> usize {
        self.template.len()
    }

    pub fn template_string(&self) -> &str {
        &self.template
    }

    pub fn execute<I, T>(&self, items: I) -> String
    where
        I: IntoIterator<Item = T>,
        T: std::fmt::Display,
    {
        let mut output = self.template.clone();
        let mut offset = 0;

        for (idx, item) in self.args.iter().copied().zip(items) {
            let item = item.to_string();
            output.insert_str(idx + offset, &item);
            offset += item.len();
        }

        output
    }
}

impl<'plan> LayoutArgPlan<'plan> {
    pub fn placeholder(&mut self) {
        let idx = self.plan.template.len();
        self.plan.args.push(idx);
    }
}

impl std::fmt::Write for LayoutPlan {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.template.write_str(s)
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.template.write_char(c)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.template.write_fmt(args)
    }
}

impl<'plan> std::fmt::Write for LayoutArgPlan<'plan> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.plan.template.write_str(s)
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.plan.template.write_char(c)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.plan.template.write_fmt(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn finds_repeated_prefix() {
        assert_eq!(get_repeated_prefix("*"), ("*", ""));
        assert_eq!(get_repeated_prefix("**"), ("**", ""));
        assert_eq!(get_repeated_prefix("only one"), ("o", "nly one"));
        assert_eq!(get_repeated_prefix("**hello**"), ("**", "hello**"));
        assert_eq!(get_repeated_prefix("___wow___"), ("___", "wow___"));
    }

    #[test]
    pub fn substr_markdown_retains_bold_italics() {
        assert_eq!(
            substr_markdown("*hello world*", 6),
            SubstrAttempt {
                markdown: "*hello world*",
                start_of_token: Some(0)
            }
        );

        assert_eq!(
            substr_markdown("**hello** _world_", 6),
            SubstrAttempt {
                markdown: "**hello**",
                start_of_token: Some(0)
            }
        );

        assert_eq!(
            substr_markdown("hello _world_", 8),
            SubstrAttempt {
                markdown: "hello _world_",
                start_of_token: Some(6)
            }
        );
    }

    #[test]
    pub fn substr_markdown_retains_mixed_bold_italic() {
        assert_eq!(
            substr_markdown("**_hello_ world**", 6),
            SubstrAttempt {
                markdown: "**_hello_ world**",
                start_of_token: Some(0)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_ignores_single_tilde() {
        assert_eq!(
            substr_markdown("~hello~", 3),
            SubstrAttempt {
                markdown: "~he",
                start_of_token: None
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_strikethrough() {
        assert_eq!(
            substr_markdown("dont ~~remove this~~", 6),
            SubstrAttempt {
                markdown: "dont ~~remove this~~",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_spoilers() {
        assert_eq!(
            substr_markdown("dont >!remove this!<", 6),
            SubstrAttempt {
                markdown: "dont >!remove this!<",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_single_superscript() {
        assert_eq!(
            substr_markdown("dont ^remove this", 6),
            SubstrAttempt {
                markdown: "dont ^remove",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_single_superscript_with_formatting() {
        assert_eq!(
            substr_markdown("dont ^*remove* this", 6),
            SubstrAttempt {
                markdown: "dont ^*remove*",
                start_of_token: Some(5)
            }
        );

        assert_eq!(
            substr_markdown("dont ^*remove* this", 17),
            SubstrAttempt {
                markdown: "dont ^*remove* th",
                start_of_token: None
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_grouped_superscript() {
        assert_eq!(
            substr_markdown("dont ^(remove this), but do this", 6),
            SubstrAttempt {
                markdown: "dont ^(remove this)",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_inline_code() {
        assert_eq!(
            substr_markdown("dont `remove this`, but do this", 6),
            SubstrAttempt {
                markdown: "dont `remove this`",
                start_of_token: Some(5)
            }
        );

        assert_eq!(
            substr_markdown("dont `remove this`, but do this", 22),
            SubstrAttempt {
                markdown: "dont `remove this`, bu",
                start_of_token: None
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_user_subreddit_shortlinks() {
        assert_eq!(
            substr_markdown("dont u/removethis, but do this", 8),
            SubstrAttempt {
                markdown: "dont u/removethis",
                start_of_token: Some(5)
            }
        );

        assert_eq!(
            substr_markdown("do not r/removethis, but do this", 10),
            SubstrAttempt {
                markdown: "do not r/removethis",
                start_of_token: Some(7)
            }
        );

        assert_eq!(
            substr_markdown("dont u/removethis", 8),
            SubstrAttempt {
                markdown: "dont u/removethis",
                start_of_token: Some(5)
            }
        );

        assert_eq!(
            substr_markdown("do not r/removethis", 10),
            SubstrAttempt {
                markdown: "do not r/removethis",
                start_of_token: Some(7)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_markdown_links() {
        assert_eq!(
            substr_markdown(
                "dont [remove this](https://example.com), but do remove this",
                6
            ),
            SubstrAttempt {
                markdown: "dont [remove this](https://example.com)",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn strstr_markdown_retains_raw_links() {
        assert_eq!(
            substr_markdown("dont https://removethis.example.com, but do remove this", 6),
            SubstrAttempt {
                markdown: "dont https://removethis.example.com",
                start_of_token: Some(5)
            }
        );

        assert_eq!(
            substr_markdown(
                "dont <https://removethis.example.com>, but do remove this",
                6
            ),
            SubstrAttempt {
                markdown: "dont <https://removethis.example.com>",
                start_of_token: Some(5)
            }
        );
    }

    #[test]
    pub fn substr_markdown_short_gives_extra_to_long() {
        let text = vec![
            "short0",
            "short1",
            "short2",
            "short3",
            "short4",
            "a very long piece of text that would otherwise be truncated",
        ];

        let total = text.iter().map(|v| v.len()).sum::<usize>();

        let result = super::substr_markdown_many(text.iter().copied(), total);

        for (item, substr) in text.iter().copied().zip(result) {
            assert_eq!(substr.markdown, item);
        }
    }

    #[test]
    pub fn substr_markdown_many_trims() {
        let text = vec![
            "hello _world_",
            "this **is some** text",
            "what <https://example.com>",
        ];

        let result = substr_markdown_many(text.into_iter(), 27);

        assert_eq!(
            result,
            vec![
                SubstrAttempt {
                    markdown: "hello ",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "this **is some**",
                    start_of_token: Some(5),
                },
                SubstrAttempt {
                    markdown: "what ",
                    start_of_token: None,
                }
            ]
        )
    }

    #[test]
    pub fn substr_markdown_many_unaffected_large_limit() {
        let text = vec![
            "hello _world_",
            "this **is some** text",
            "what <https://example.com>",
        ];

        let result = substr_markdown_many(text.into_iter(), 10_000);

        assert_eq!(
            result,
            vec![
                SubstrAttempt {
                    markdown: "hello _world_",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "this **is some** text",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "what <https://example.com>",
                    start_of_token: None,
                }
            ]
        )
    }

    #[test]
    pub fn substrs_markdown_fix_panic_triggered() {
        let text = vec!["1234567890".repeat(10), String::from("_Old_  \n**New**")];

        let result = substr_markdown_many(text.iter().map(|v| v.as_str()), 30);

        assert_eq!(
            result,
            vec![
                SubstrAttempt {
                    markdown: "123456789012345",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "_Old_  \n**New**",
                    start_of_token: None,
                },
            ]
        )
    }
}
