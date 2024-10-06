use crate::{analysis::DetectedItem, utils::Words};

use super::Matcher;

#[derive(Debug, PartialEq)]
pub struct ExactMatcher(Words);

impl ExactMatcher {
    pub fn new(words: impl Into<String>) -> Self {
        let words = Words::new(words);
        Self(words)
    }
}

impl Matcher for ExactMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Option<crate::analysis::DetectedItem> {
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
                return Some(item);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{analysis::str_matchers::Matcher, utils::Words};

    use super::ExactMatcher;

    #[test]
    pub fn test_exact_matcher() {
        let exact = ExactMatcher::new("quick brown fox");
        let test = Words::new("hello world the quick brown fox jumped over the lazy dog");

        let result = exact.matches(&test.as_words(), true).unwrap();

        assert_eq!(result.words.len(), 3);
        assert!(result.words.contains_key(&3));
        assert!(result.words.contains_key(&4));
        assert!(result.words.contains_key(&5));
    }
}
