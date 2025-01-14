use serde::Deserialize;

use crate::analysis::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct OrderedMatcher(pub Vec<MatcherKind>);

fn recursive_matches(
    words: &[&str],
    matchers: &[MatcherKind],
    debug: bool,
    base: DetectedItem,
    start_idx: usize,
) -> Vec<DetectedItem> {
    if matchers.len() == 0 {
        return vec![base];
    }

    let next = &matchers[0];

    let result = next.matches(&words[start_idx..], debug);
    if result.len() == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();

    for result in result {
        let mut this = base.clone();
        for (idx, word) in result.words {
            this.words.insert(start_idx + idx, word);
        }
        this.score += result.score;

        let (min, _) = this.min_max_word_indexes();

        let next = recursive_matches(words, &matchers[1..], debug, this, min);
        for next in next {
            results.push(next);
        }
    }

    results
}

impl Matcher for OrderedMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        let mut v = recursive_matches(words, &self.0, debug, DetectedItem::new(0.0), 0);
        v.retain_mut(|d| {
            d.score /= self.0.len() as f32;
            d.score > 0.8
        });
        v
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        analysis::str_matchers::{Matcher, MatcherKind, PhraseMatcher},
        utils::Words,
    };

    use super::OrderedMatcher;

    #[test]
    pub fn test_multi_match() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("hello there hello some other world");
        let words = text.as_words();

        let result = ordered.matches(&words, true);

        assert_eq!(result.len(), 2);

        let mut opt_one = Some("**hello** there hello some other **world**");
        let mut opt_two = Some("hello there **hello** some other **world**");

        for result in result {
            let mut s = String::with_capacity(text.full_text().len());
            result.write_markdown(&text.as_words(), &mut s).unwrap();
            if let Some(opt) = &opt_one {
                if s == *opt {
                    opt_one = None;
                    continue;
                }
            }
            if let Some(opt) = &opt_two {
                if s == *opt {
                    opt_two = None;
                    continue;
                }
            }
            panic!("unrecognised {s:?}");
        }
        assert_eq!(opt_one, None);
        assert_eq!(opt_two, None);
    }

    #[test]
    pub fn test_matches() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("hello there some other world");
        let words = text.as_words();

        let det = &ordered.matches(&words, true)[0];

        let mut s = String::with_capacity(text.full_text().len());
        det.write_markdown(&words, &mut s).unwrap();

        assert_eq!(s, "**hello** there some other **world**");
    }

    #[test]
    pub fn test_no_match() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
            MatcherKind::Phrase(PhraseMatcher::new("another")),
        ]);

        let text = Words::new("hello there some other world");
        let words = text.as_words();

        let det = ordered.matches(&words, true);

        assert!(det.is_empty())
    }

    #[test]
    pub fn test_follows_ordering() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("world hello");
        let words = text.as_words();

        let det = ordered.matches(&words, true);

        assert!(det.is_empty())
    }
}
