use mlapibot_common::DetectedItem;
use serde::Deserialize;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct AllMatcher {
    children: Vec<MatcherKind>,
}

impl AllMatcher {
    pub fn new(children: Vec<MatcherKind>) -> Self {
        Self { children }
    }
}

impl Matcher for AllMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        let mut item = DetectedItem::new(0.0);

        for child in &self.children {
            match child.best_match(words, debug) {
                Some(m) => item += m,
                None => return Vec::new(),
            }
        }

        // make it an average.
        item.score /= self.children.len() as f32;

        vec![item]
    }
}
