use chrono::Utc;
use serde::Deserialize;

use crate::{
    utils::{into_timestamp, LowercaseString},
    webhook::{create_change_flair_message, WebhookClient},
};

use super::{subreddit::Subreddit, RedditClient, Submission};

pub type SubredditFlairConfig = Vec<FlairChangeConfig>;

#[derive(Debug, Deserialize)]
pub struct FlairChangeConfig {
    pub flair_id: String,
    pub title_must_include: LowercaseString,
    pub permitted_user_flairs: Vec<String>,
    pub change_to: String,
}

impl RedditClient<'_> {
    pub(super) fn check_post_flairs(
        dry_run: bool,
        subreddit: &Subreddit,
        post: &Submission,
        webhook: &mut Option<WebhookClient>,
        flairs: &SubredditFlairConfig,
    ) -> anyhow::Result<()> {
        if subreddit.is_moderator(post.author().as_str()) {
            return Ok(());
        }

        let utc = into_timestamp(post.created_utc());
        let diff = Utc::now() - utc;
        if diff.abs().num_seconds() < 30 {
            // delay to ignore any posts immediately removed by AutoMod.
            return Ok(());
        }

        let post_flair_id = match post.link_flair_template_id() {
            Some(id) => id.as_str(),
            None => return Ok(()),
        };

        let title_lowercase = post.title().to_lowercase();

        for flair in flairs {
            if flair.flair_id != post_flair_id {
                continue;
            }

            if let Some(author_flair) = post.author_flair_template_id() {
                if flair
                    .permitted_user_flairs
                    .iter()
                    .any(|f| f == author_flair)
                {
                    continue;
                }
            }

            if title_lowercase.contains(flair.title_must_include.as_str()) {
                continue;
            }

            // At this point, the post uses the flair, the author is not exempt, and the title is not well-formed.
            // Change the flair accordingly.

            if !dry_run {
                post.select_flair(&flair.change_to)?;
                if let Some(webhook) = webhook {
                    webhook.send(&create_change_flair_message(post, &flair.change_to))?;
                }
                break;
            }
        }

        Ok(())
    }
}
