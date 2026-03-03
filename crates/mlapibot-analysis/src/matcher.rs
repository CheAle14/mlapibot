mod all;
mod any;
mod exact;
mod ordered;
mod phrase;

use mlapibot_common::{DetectedItem, Words, matchers::Matchers};

pub trait Matcher {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem>;

    fn best_match(&self, words: &[&str], debug: bool) -> Option<DetectedItem> {
        let mut all = self.matches(words, debug);
        all.sort_unstable();
        all.into_iter().next()
    }

    fn any_matches(&self, ctx: &crate::context::Context) -> bool {
        for img in &ctx.images {
            let words = img.words();
            if self.matches(&words, ctx.debug).len() > 0 {
                return true;
            }
        }
        if let Some(title) = &ctx.title {
            let words = Words::new(title);
            let words = words.as_words();
            if self.matches(&words, ctx.debug).len() > 0 {
                return true;
            }
        }
        if let Some(body) = &ctx.body {
            let words = Words::new(body);
            let words = words.as_words();
            if self.matches(&words, ctx.debug).len() > 0 {
                return true;
            }
        }

        false
    }
}

impl Matcher for Matchers {
    fn matches(&self, words: &[&str], debug: bool) -> Vec<DetectedItem> {
        match &self {
            Matchers::Phrase(v) => v.matches(words, debug),
            Matchers::Ordered(v) => v.matches(words, debug),
            Matchers::Any(v) => v.matches(words, debug),
            Matchers::All(v) => v.matches(words, debug),
            Matchers::Exact(v) => v.matches(words, debug),
        }
    }
}
