use serde::Deserialize;

use crate::analysis::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct OrderedMatcher(pub Vec<MatcherKind>);

impl Matcher for OrderedMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Option<crate::analysis::DetectedItem> {
        let mut start_idx = 0;

        let mut score_sum = 0.0;
        let mut detection = DetectedItem::new(0.0);

        for child in self.0.iter() {
            let words = &words[start_idx..];
            if let Some(next) = child.matches(words, debug) {
                score_sum += next.score;
                let (_, max) = next.min_max_word_indexes();
                for (key, value) in next.words {
                    detection.words.insert(key + start_idx, value);
                }

                start_idx = max;
            } else {
                return None;
            }
        }

        detection.score = score_sum / self.0.len() as f32;
        Some(detection)
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
    pub fn test_matches() {
        let ordered = OrderedMatcher(vec![
            MatcherKind::Phrase(PhraseMatcher::new("hello")),
            MatcherKind::Phrase(PhraseMatcher::new("world")),
        ]);

        let text = Words::new("hello there some other world");
        let words = text.as_words();

        let det = ordered.matches(&words, true).unwrap();

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

        assert!(det.is_none())
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

        assert!(det.is_none())
    }
}
