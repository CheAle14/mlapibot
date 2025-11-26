use linkify::{Link, LinkKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstrAttempt<'md> {
    markdown: &'md str,
    /// If present, the position of the item that forced `markdown` to be larger
    /// than the requested `max`.
    start_of_token: Option<usize>,
}

impl<'md> SubstrAttempt<'md> {
    fn token(markdown: &'md str, start_of_token: usize) -> Self {
        Self {
            markdown,
            start_of_token: Some(start_of_token),
        }
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

/// Attempts to shorten each of the items in the iterator until their total length is below max.
///
/// The returned vec contains each item provided by the `items` iterator in the same order.
pub fn substr_markdown_many<'md>(
    items: impl Iterator<Item = &'md str>,
    max: usize,
) -> Vec<SubstrAttempt<'md>> {
    let mut substrs: Vec<_> = items
        .map(|markdown| {
            (
                markdown,
                SubstrAttempt {
                    markdown,
                    start_of_token: None,
                },
            )
        })
        .collect();

    'item_max: for items_max in (0..(max.div_ceil(substrs.len()))).rev() {
        let mut current_sum = 0;
        for sub in &mut substrs {
            sub.1 = substr_markdown(sub.0, items_max);
            current_sum += sub.1.markdown.len();
        }

        while current_sum > max {
            let global_distance = current_sum - max;
            println!(
                "{current_sum} > {max} (= {global_distance}) (for each {items_max}); {substrs:?}"
            );

            let mut closest_to_distance: Option<(isize, &mut SubstrAttempt)> = None;
            for substr in &mut substrs {
                let Some(removable) = substr.1.removalable_len() else {
                    continue;
                };

                let distance_to_global = removable
                    .checked_signed_diff(global_distance)
                    .expect("never overflows");

                println!(
                    "can remove {removable} ({distance_to_global}) for {:?}",
                    substr.1.markdown
                );

                match closest_to_distance {
                    Some((r, _)) => {
                        if distance_to_global > r {
                            closest_to_distance = Some((distance_to_global, &mut substr.1));
                        }
                    }
                    None => closest_to_distance = Some((distance_to_global, &mut substr.1)),
                }
            }

            match closest_to_distance {
                Some((_, substr)) => {
                    let new = substr.trim_to_token().expect("removable not None");
                    let diff = substr.len() - new.len();
                    println!("removed {diff} to {:?}", new.markdown);
                    current_sum -= diff;
                    *substr = new;
                }
                None => {
                    println!(
                        "no more to remove: {current_sum} for {max}, {items_max}: {substrs:?}"
                    );

                    continue 'item_max;
                }
            }
        }

        println!("achived {current_sum}");
        for (original, item) in &substrs {
            println!("- {} to {}", original.len(), item.len());
        }

        return substrs.into_iter().map(|(_, a)| a).collect();
    }

    panic!("unable to perform *any* reduction?? even empty strings should satisfy!");
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
    pub fn substr_markdown_many_trims() {
        let text = vec![
            "hello _world_",
            "this **is some** text",
            "what <https://example.com>",
        ];

        let result = substr_markdown_many(text.into_iter(), 21);

        assert_eq!(
            result,
            vec![
                SubstrAttempt {
                    markdown: "hello ",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "this ",
                    start_of_token: None,
                },
                SubstrAttempt {
                    markdown: "what ",
                    start_of_token: None,
                }
            ]
        )
    }
}
