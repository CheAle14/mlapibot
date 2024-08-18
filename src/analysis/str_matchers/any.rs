use crate::analysis::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct AnyMatcher(pub Vec<MatcherKind>);

impl Matcher for AnyMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Option<crate::analysis::DetectedItem> {
        let mut best: Option<DetectedItem> = None;

        for child in self.0.iter() {
            if let Some(next) = child.matches(words, debug) {
                if best.is_none() || best.as_ref().unwrap().score < next.score {
                    best = Some(next);
                }
            }
        }

        best
    }
}
