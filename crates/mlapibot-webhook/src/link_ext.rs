use std::borrow::Cow;

use crate::embed::MessageEmbed;

pub fn as_reddit_link(text: &str) -> Cow<'_, str> {
    if text.starts_with("http") {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(format!("https://reddit.com{text}"))
    }
}

pub trait LinkExt {
    fn with_reddit_link(&mut self, maybe_full_url: &str) -> &mut Self;
    fn reddit_link(mut self, maybe_full_url: &str) -> Self
    where
        Self: Sized,
    {
        self.with_reddit_link(maybe_full_url);
        self
    }
}

impl LinkExt for MessageEmbed {
    fn with_reddit_link(&mut self, maybe_full_url: &str) -> &mut Self {
        self.with_url(as_reddit_link(maybe_full_url))
    }
}
