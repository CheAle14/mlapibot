use mlapibot_common::{DetectedItem, matchers::AllMatcher};

use super::Matcher;

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
