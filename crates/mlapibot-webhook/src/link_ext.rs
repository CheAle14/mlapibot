use crate::embed::MessageEmbed;

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
        if maybe_full_url.starts_with("http") {
            self.with_url(maybe_full_url)
        } else {
            let full = format!("https://reddit.com{maybe_full_url}");
            self.with_url(full)
        }
    }
}
