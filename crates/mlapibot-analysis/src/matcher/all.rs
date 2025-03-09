use mlapibot_common::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct AllMatcher(pub Vec<MatcherKind>);

impl Matcher for AllMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        let mut item = DetectedItem::new(0.0);

        for child in &self.0 {
            match child.best_match(words, debug) {
                Some(m) => item += m,
                None => return Vec::new(),
            }
        }

        // make it an average.
        item.score /= self.0.len() as f32;

        vec![item]
    }
}
