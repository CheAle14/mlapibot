use crate::analysis::DetectedItem;

use super::{Matcher, MatcherKind};

#[derive(Debug, PartialEq)]
pub struct AllMatcher(pub Vec<MatcherKind>);

impl Matcher for AllMatcher {
    fn matches(&self, words: &[&str], debug: bool) -> Option<crate::analysis::DetectedItem> {
        let mut item = DetectedItem::new(0.0);

        for child in &self.0 {
            match child.matches(words, debug) {
                Some(mtch) => {
                    item += mtch;
                }
                None => return None,
            }
        }

        // make it an average.
        item.score /= self.0.len() as f32;

        Some(item)
    }
}
