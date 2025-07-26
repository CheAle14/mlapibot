use std::collections::HashSet;

use mlapibot_common::Words;
use roux::{builders::submission::SubmissionSubmitBuilder, client::RedditClient};

pub struct PostRelatedTitle;

impl super::Module for PostRelatedTitle {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "post_related_title"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::POSTS
    }

    super::impl_mask_subreddits!(related_titles => posts);

    fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<()> {
        if has_seen {
            return Ok(());
        }

        if !post.is_self() || post.selftext().trim().len() == 0 {
            return Ok(());
        }

        if post.selftext().contains("http") {
            // Image could contain more context.
            return Ok(());
        }

        let mut title_words = Words::new(post.title());
        title_words.remove_stop_words();

        let mut body_words = Words::new(post.selftext());
        body_words.remove_stop_words();

        let title_words = title_words.iter_stemmed_words().collect::<HashSet<_>>();
        let body_words = body_words.iter_stemmed_words().collect::<HashSet<_>>();

        let both = title_words.intersection(&body_words).count();

        if both == 0 {
            client
                .client
                .subreddit("mlapi")
                .submit(&SubmissionSubmitBuilder::link(
                    "Vague title",
                    format!("https://reddit.com{}", post.permalink()),
                    false,
                ))?;
        }

        Ok(())
    }
}
