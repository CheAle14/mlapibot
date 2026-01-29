use std::collections::HashSet;

use mlapibot_common::Words;

use crate::client::module::{ActionData, PostAction};

pub struct PostVagueTitle;

#[async_trait::async_trait(?Send)]
impl super::Module for PostVagueTitle {
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

    async fn run_post<'client>(
        &mut self,
        _client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        if has_seen {
            return Ok(PostAction::Ignore);
        }

        if !post.is_self() || post.selftext().trim().len() == 0 {
            return Ok(PostAction::Ignore);
        }

        if post.selftext().contains("http") {
            // Image could contain more context.
            return Ok(PostAction::Ignore);
        }

        let mut title_words = Words::new(post.title());
        title_words.remove_stop_words();

        let title_words = title_words.iter_stemmed_words().collect::<HashSet<_>>();

        if title_words.len() > 0 {
            return Ok(PostAction::Ignore);
        }

        let modconf = config.and_then(|c| c.moderate.as_ref());

        let Some(modconf) = modconf else {
            return Ok(PostAction::Ignore);
        };

        let reason_id = modconf
            .removal_reasons
            .get("vague-title")
            .map(String::as_str)
            .unwrap_or_else(|| &modconf.default_removal_reason);

        let reason = subreddit
            .get_removal_reason(reason_id)
            .await?
            .map(|r| r.message.as_str())
            .unwrap_or("<error: removal reason not found>");

        Ok(PostAction::Action(
            ActionData::new().remove().reply(reason.to_string(), true),
        ))
    }
}
