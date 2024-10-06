mod all;
mod any;
mod exact;
mod ordered;
mod phrase;

pub use all::*;
pub use any::*;
pub use exact::*;
pub use ordered::*;
pub use phrase::*;
use serde::{de::Visitor, Deserialize};

use crate::{analysis::DetectedItem, utils::Words};

#[derive(Debug, PartialEq)]
pub enum MatcherKind {
    Phrase(PhraseMatcher),
    Ordered(OrderedMatcher),
    All(AllMatcher),
    Any(AnyMatcher),
    Exact(ExactMatcher),
}

struct MatcherKindVisitor;

impl<'de> Visitor<'de> for MatcherKindVisitor {
    type Value = MatcherKind;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string, seq, map")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MatcherKind::Phrase(PhraseMatcher::new(v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MatcherKind::Phrase(PhraseMatcher::new(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut kinds = if let Some(size) = seq.size_hint() {
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };

        while let Some(next) = seq.next_element()? {
            kinds.push(next);
        }

        Ok(MatcherKind::Any(AnyMatcher(kinds)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;

        let mut tag = None;
        let mut children = None;
        let mut phrase = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => tag = Some(map.next_value::<String>()?),
                "children" => children = Some(map.next_value::<Vec<MatcherKind>>()?),
                "phrase" => phrase = Some(map.next_value::<String>()?),
                other => {
                    return Err(A::Error::unknown_field(
                        other,
                        &["type", "children", "phrase"],
                    ))
                }
            }
        }

        let tag = tag.expect("'type' field set");

        match tag.as_str() {
            "ordered" => Ok(MatcherKind::Ordered(OrderedMatcher(
                children.expect("'children' set for ordered"),
            ))),
            "any" => Ok(MatcherKind::Any(AnyMatcher(
                children.expect("'children' set for any"),
            ))),
            "all" => Ok(MatcherKind::All(AllMatcher(
                children.expect("'children' set for all"),
            ))),
            "exact" => Ok(MatcherKind::Exact(ExactMatcher::new(
                phrase.expect("'phrase' set for exact"),
            ))),
            s => Err(A::Error::unknown_variant(
                s,
                &["ordered", "any", "all", "exact"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for MatcherKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(MatcherKindVisitor)
    }
}

pub trait Matcher {
    fn matches(&self, words: &[&str], debug: bool) -> Option<DetectedItem>;

    fn any_matches(&self, ctx: &crate::context::Context) -> bool {
        for img in &ctx.images {
            let words = img.words();
            if self.matches(&words, ctx.debug).is_some() {
                return true;
            }
        }
        if let Some(title) = &ctx.title {
            let words = Words::new(title);
            let words = words.as_words();
            if self.matches(&words, ctx.debug).is_some() {
                return true;
            }
        }
        if let Some(body) = &ctx.body {
            let words = Words::new(body);
            let words = words.as_words();
            if self.matches(&words, ctx.debug).is_some() {
                return true;
            }
        }

        false
    }
}

impl Matcher for MatcherKind {
    fn matches(&self, words: &[&str], debug: bool) -> Option<DetectedItem> {
        match &self {
            MatcherKind::Phrase(v) => v.matches(words, debug),
            MatcherKind::Ordered(v) => v.matches(words, debug),
            MatcherKind::Any(v) => v.matches(words, debug),
            MatcherKind::All(v) => v.matches(words, debug),
            MatcherKind::Exact(v) => v.matches(words, debug),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::str_matchers::{AnyMatcher, MatcherKind, OrderedMatcher, PhraseMatcher};

    #[test]
    pub fn deserialize_string() {
        const JSON: &str = r#""hello world""#;

        let parsed: MatcherKind = serde_json::from_str(JSON).unwrap();
        assert_eq!(
            parsed,
            MatcherKind::Phrase(PhraseMatcher::new("hello world"))
        );
    }

    #[test]
    pub fn deserialize_vec_to_any() {
        const JSON: &str = r#"["hello world", "another one"]"#;

        let parsed: MatcherKind = serde_json::from_str(JSON).unwrap();
        assert_eq!(
            parsed,
            MatcherKind::Any(AnyMatcher(vec![
                MatcherKind::Phrase(PhraseMatcher::new("hello world")),
                MatcherKind::Phrase(PhraseMatcher::new("another one"))
            ]))
        );
    }

    #[test]
    pub fn deserialize_ordered() {
        const JSON: &str = r#"{"type":"ordered", "children":["hello world", "another one"]}"#;

        let parsed: MatcherKind = serde_json::from_str(JSON).unwrap();
        assert_eq!(
            parsed,
            MatcherKind::Ordered(OrderedMatcher(vec![
                MatcherKind::Phrase(PhraseMatcher::new("hello world")),
                MatcherKind::Phrase(PhraseMatcher::new("another one"))
            ]))
        );
    }

    #[test]
    pub fn deserialize_recursive() {
        const JSON: &str = r#"{
            "type":"ordered", 
            "children":[
                [
                    "hello world", "another one"
                ],
                {
                    "type": "ordered",
                    "children": [
                        "other",
                        "text",
                        "goes",
                        "here"
                    ]
                }
            ]
        }"#;

        let parsed: MatcherKind = serde_json::from_str(JSON).unwrap();
        assert_eq!(
            parsed,
            MatcherKind::Ordered(OrderedMatcher(vec![
                MatcherKind::Any(AnyMatcher(vec![
                    MatcherKind::Phrase(PhraseMatcher::new("hello world")),
                    MatcherKind::Phrase(PhraseMatcher::new("another one"))
                ])),
                MatcherKind::Ordered(OrderedMatcher(vec![
                    MatcherKind::Phrase(PhraseMatcher::new("other")),
                    MatcherKind::Phrase(PhraseMatcher::new("text")),
                    MatcherKind::Phrase(PhraseMatcher::new("goes")),
                    MatcherKind::Phrase(PhraseMatcher::new("here"))
                ]))
            ]))
        );
    }
}
