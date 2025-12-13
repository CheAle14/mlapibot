use mlapibot_common::{DetectedItem, Words};

use super::Matcher;

#[derive(Debug, PartialEq, Clone)]
pub struct ExactMatcher(Words);

impl ExactMatcher {
    pub fn new(words: impl Into<String>) -> Self {
        let words = Words::new(words);
        Self(words)
    }
}

impl Matcher for ExactMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        let check_words = self.0.as_words();
        for (start_idx, window) in words.windows(self.0.len()).enumerate() {
            if (&check_words) == window {
                if debug {
                    println!("Exact match at {start_idx}");
                }
                let mut item = DetectedItem::new(1.0);
                for i in start_idx..start_idx + check_words.len() {
                    item.mark_match(i);
                }
                return vec![item];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use mlapibot_common::Words;

    use crate::matcher::Matcher;

    use super::ExactMatcher;

    #[test]
    pub fn test_exact_matcher() {
        let exact = ExactMatcher::new("quick brown fox");
        let test = Words::new("hello world the quick brown fox jumped over the lazy dog");

        let result = &exact.matches(&test.as_words(), true)[0];

        assert_eq!(result.words.len(), 3);
        assert!(result.words.contains_key(&3));
        assert!(result.words.contains_key(&4));
        assert!(result.words.contains_key(&5));
    }

    #[test]
    pub fn test_exact_match_uppercase() {
        let exact = ExactMatcher::new("quick brown fox");
        let test = Words::new("hello world the QUICK brOWN Fox jumped over the lazy dog");

        let result = &exact.matches(&test.as_words(), true)[0];

        assert_eq!(result.words.len(), 3);
        assert!(result.words.contains_key(&3));
        assert!(result.words.contains_key(&4));
        assert!(result.words.contains_key(&5));
    }
}
