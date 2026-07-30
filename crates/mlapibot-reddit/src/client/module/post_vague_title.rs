use std::collections::HashSet;

use anyhow::Context;
use mlapibot_common::{
    Words,
    action::{ActionData, PostAction},
};
use mlapibot_database_v2::repos::subreddits::SubredditsRepo;
use mlapibot_webhook::{Message, MessageEmbed, as_reddit_link};
use roux::{api::Distinguished, client::AuthedClient, models::LatestComment};

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
        super::ModuleWants::POSTS | super::ModuleWants::COMMENTS
    }

    fn mask_subreddits(&self, subreddits: &[crate::subreddit::Subreddit]) -> super::SplitSubMask {
        let mut sum = super::SplitSubMask::new();
        for (idx, sub) in subreddits.iter().enumerate() {
            if sub.db.mod_related_title.enabled {
                sum.posts.set(idx);

                if sub.db.mod_related_title.auto_add_remove_text.is_some() {
                    sum.comments.set(idx);
                }
            }
        }

        sum
    }

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
        title_words.remove_stop_words(|word| subreddit.vague_words.contains(word));

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

    async fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        comment: &LatestComment<AuthedClient>,
    ) -> anyhow::Result<()> {
        let Some(auto) = subreddit.db.mod_related_title.auto_add_remove_text.as_ref() else {
            return Ok(());
        };

        if comment.parent_id() != comment.link_id() {
            // only top-level removal reasons for the post
            return Ok(());
        }

        if !matches!(comment.distinguished(), Distinguished::Moderator) {
            return Ok(());
        }

        if !comment.body().contains(auto) {
            println!("[vague-title-auto] body did not contain {auto:?}");
            return Ok(());
        }

        let mut words = Words::new(comment.link_title());
        words.remove_stop_words(|word| subreddit.vague_words.contains(word));

        if words.len() == 0 {
            println!("[vague-title-auto] all words already marked as vague");
            return Ok(());
        }

        let words = words.as_words();

        client
            .db
            .add_vague_words(&subreddit.db.id, &words)
            .await
            .with_context(|| {
                format!(
                    "adding {} vague words for {}",
                    words.len(),
                    subreddit.db.name
                )
            })?;

        for &word in &words {
            subreddit.vague_words.add(word.to_owned());
        }

        if let Some(webhook) = client.webhook.as_mut() {
            let mut text = String::from("```\n");
            for word in &words {
                text.push_str(*word);
                text.push_str(", ");
            }
            text.pop();
            text.pop();
            text.push_str("\n```");

            webhook
                .send(
                    &Message::builder().embed(
                        MessageEmbed::builder()
                            .title("Auto-add vague words")
                            .description(text)
                            .field(
                                "Comment",
                                format!(
                                    "[{}]({})",
                                    comment.author(),
                                    as_reddit_link(comment.permalink())
                                ),
                            ),
                    ),
                )
                .await?;
        }

        Ok(())
    }
}
