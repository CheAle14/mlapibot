use mlapibot_common::{DetectedItem, matchers::AnyMatcher};

use super::Matcher;

impl Matcher for AnyMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        self.children
            .iter()
            .map(|a| a.matches(words, debug))
            .flatten()
            .collect()
    }
}
