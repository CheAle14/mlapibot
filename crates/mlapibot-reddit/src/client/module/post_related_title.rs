use mlapibot_common::Words;

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

        let title_words = title_words.as_hash_set();
        let body_words = body_words.as_hash_set();

        let both = title_words.intersection(&body_words).count();

        if both == 0 {
            post.report("possible vague title (no words in title appear in body)")?;
        }

        Ok(())
    }
}
