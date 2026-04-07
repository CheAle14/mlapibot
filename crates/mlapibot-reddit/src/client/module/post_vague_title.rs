use std::collections::HashSet;

use mlapibot_common::{
    Words,
    action::{ActionData, PostAction},
};

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

    super::impl_mask_subreddits!(mod_related_title => posts);

    async fn run_post<'client>(
        &mut self,
        _client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        if has_seen {
            return Ok(PostAction::Ignore);
        }

        let is_image_post = !post.is_self() || post.selftext().trim().len() == 0;
        if is_image_post && !subreddit.db.mod_related_title.check_img_posts {
            return Ok(PostAction::Ignore);
        }

        let mut title_words = Words::new(post.title());
        title_words.remove_stop_words();

        let title_words = title_words.iter_stemmed_words().collect::<HashSet<_>>();

        if title_words.len() > 0 {
            return Ok(PostAction::Ignore);
        }

        let Some(reason_id) = subreddit
            .db
            .removal_reasons
            .get(&subreddit.db.mod_related_title.reason)
        else {
            eprintln!(
                "related_title removal reason is invalid: {}",
                subreddit.name()
            );
            return Ok(PostAction::Ignore);
        };

        let reason = subreddit
            .removal_reasons
            .data(&subreddit.reddit)
            .await?
            .get(reason_id)
            .map(|r| r.message.as_str())
            .unwrap_or("<error: removal reason not found>");

        Ok(PostAction::Action(
            ActionData::new().remove().reply(reason.to_string(), true),
        ))
    }
}
