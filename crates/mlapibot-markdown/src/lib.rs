use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct CodeFence<'text> {
    pub info_string: Option<&'text str>,
    pub code: &'text str,
    pub leading_spaces: usize,
}

impl<'text> CodeFence<'text> {
    pub fn new(leading_spaces: usize, inside_block: &'text str) -> Self {
        let (info_string, rest) = match inside_block.split_once('\n') {
            Some((before, after)) => {
                let trimmed = before.trim_ascii();
                if trimmed.len() > 0 {
                    (Some(trimmed), after)
                } else {
                    (None, after)
                }
            }
            _ => (None, inside_block),
        };

        let rest = rest.trim_end_matches(['\r', '\n']);

        Self {
            info_string,
            code: rest,
            leading_spaces,
        }
    }
}

impl<'text> std::fmt::Display for CodeFence<'text> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(lang) = self.info_string {
            writeln!(f, "    // {lang}")?;
        }

        let leading = " ".repeat(self.leading_spaces);

        for line in self.code.lines() {
            let line = line.trim_start_matches(&leading);

            writeln!(f, "    {line}")?;
        }

        Ok(())
    }
}

pub fn extract_code_fences(text: &str) -> Vec<CodeFence> {
    let mut fences = Vec::new();

    let mut offset = 0;

    loop {
        if offset >= text.len() {
            break;
        }

        let remainder = &text[offset..];

        match remainder.find(['`', '~']) {
            Some(oindex) => {
                let index = oindex + offset;
                offset += oindex + 1;

                let before = &text[..index];
                let rest = &text[index..];

                let preceeding_spaces = before.len() - before.trim_end_matches(' ').len();

                let mut chars = rest.chars();
                let fence = chars.next().unwrap();
                let mut seen = 1;

                loop {
                    match chars.next() {
                        Some('~') if fence == '~' => seen += 1,
                        Some('`') if fence == '`' => seen += 1,

                        _ => break,
                    }
                }

                if seen < 3 {
                    continue;
                }

                let rest = &rest[seen..];
                offset += seen;

                let final_block = fence.to_string().repeat(seen);

                let Some((code, rest)) = rest.split_once(&final_block) else {
                    continue;
                };

                let rest_trimmed = rest.trim_start_matches(fence);
                let extra = rest.len() - rest_trimmed.len();

                offset += code.len() + final_block.len() + extra - 1;

                fences.push(CodeFence::new(preceeding_spaces, code));
            }
            None => break,
        }
    }

    fences
}

#[cfg(test)]
mod tests {
    use crate::{CodeFence, extract_code_fences};

    #[test]
    pub fn test_extra_on_end() {
        let text = r#"```
<
 >
``````
hello
```"#;

        let fences = extract_code_fences(text);

        assert_eq!(
            &fences[..],
            // only matches the one
            &[CodeFence {
                info_string: None,
                code: "<\n >",
                leading_spaces: 0
            }]
        );
    }

    #[test]
    pub fn test_extra_on_end_space_sep() {
        let text = r#"```
<
 >
``` ```
hello
```"#;

        let fences = extract_code_fences(text);

        assert_eq!(
            &fences[..],
            // only matches the one
            &[
                CodeFence {
                    info_string: None,
                    code: "<\n >",
                    leading_spaces: 0
                },
                CodeFence {
                    info_string: None,
                    code: "hello",
                    leading_spaces: 1
                }
            ]
        );
    }

    #[test]
    pub fn example_119() {
        let text = r#"```
<
 >
```"#;

        let fences = extract_code_fences(text);

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "<\n >",
                leading_spaces: 0
            }]
        );
    }

    #[test]
    pub fn test_conversion() {
        let text = r#"hello world.
        ```rust
        fn main() -> i32 {
            42
        }
        ```
        more text"#;

        let fences = extract_code_fences(text);
        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: Some("rust"),
                code: r#"        fn main() -> i32 {
            42
        }
        "#,
                leading_spaces: "        ".len()
            }]
        );

        let replaced = fences[0].to_string();

        assert_eq!(
            replaced,
            r#"    // rust
    fn main() -> i32 {
        42
    }
    
"#
        );
    }

    #[test]
    pub fn example_120() {
        let fences = extract_code_fences(
            r#"~~~
<
 >
~~~"#,
        );

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "<\n >",
                leading_spaces: 0
            }]
        );
    }

    #[test]
    pub fn example_121() {
        let fences = extract_code_fences(
            r#"``
foo
``"#,
        );

        assert_eq!(&fences[..], &[]);
    }

    #[test]
    pub fn example_122() {
        let fences = extract_code_fences(
            r#"```
aaa
~~~
```"#,
        );

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "aaa\n~~~",
                leading_spaces: 0
            }]
        );
    }
    #[test]
    pub fn example_123() {
        let fences = extract_code_fences(
            r#"~~~
aaa
```
~~~"#,
        );

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "aaa\n```",
                leading_spaces: 0
            }]
        );
    }
    #[test]
    pub fn example_124() {
        let fences = extract_code_fences(
            r#"````
aaa
```
``````"#,
        );

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "aaa\n```",
                leading_spaces: 0
            }]
        );
    }

    #[test]
    pub fn example_125() {
        let fences = extract_code_fences(
            r#"~~~~
aaa
~~~
~~~~"#,
        );

        assert_eq!(
            &fences[..],
            &[CodeFence {
                info_string: None,
                code: "aaa\n~~~",
                leading_spaces: 0
            }]
        );
    }

    //     #[test]
    //     pub fn example_127() {
    //         let fences = extract_code_fences(
    //             r#"`````

    // ```
    // aaa"#,
    //         );

    //         assert_eq!(
    //             &fences[..],
    //             &[CodeFence {
    //                 info_string: None,
    //                 code: "\n```\naaa",
    //                 leading_spaces: 0
    //             }]
    //         );
    //     }
}
