use mlapibot_common::DetectedItem;
use serde::Deserialize;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct AnyMatcher {
    children: Vec<MatcherKind>,
}

impl AnyMatcher {
    pub fn new(children: Vec<MatcherKind>) -> Self {
        Self { children }
    }
}

impl Matcher for AnyMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        self.children
            .iter()
            .map(|a| a.matches(words, debug))
            .flatten()
            .collect()
    }
}
