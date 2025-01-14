use crate::analysis::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct AnyMatcher(pub Vec<MatcherKind>);

impl Matcher for AnyMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<crate::analysis::DetectedItem> {
        self.0
            .iter()
            .map(|a| a.matches(words, debug))
            .flatten()
            .collect()
    }
}
