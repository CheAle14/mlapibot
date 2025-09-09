use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use roux::api::ThingFullname;
use serde::Deserialize;

use crate::{
    client::module::SplitSubMask, utils::into_timestamp, webhook::create_change_flair_message,
};

use mlapibot_common::LowercaseString;

pub type SubredditFlairConfig = Vec<FlairChangeConfig>;

#[derive(Debug, Deserialize)]
pub struct FlairChangeConfig {
    pub flair_id: String,
    pub title_must_include: LowercaseString,
    pub permitted_user_flairs: Vec<String>,
    pub change_to: String,
}

pub struct PostFlairData {
    pub count: u64,
    pub delay_until: DateTime<Utc>,
}

impl Default for PostFlairData {
    fn default() -> Self {
        Self {
            count: 0,
            delay_until: Utc::now(),
        }
    }
}

impl PostFlairData {
    fn delay_for(count: u64) -> Duration {
        Duration::from_secs(15 * count)
    }

    pub fn increment(&mut self) {
        self.count += 1;
        self.delay_until = Utc::now() + Self::delay_for(self.count);
    }
}

#[derive(Default)]
pub struct PostFlairCache {
    pub inner: HashMap<ThingFullname, PostFlairData>,
}

pub struct PostFlairs;

impl super::Module for PostFlairs {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "post_flairs"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::POSTS
    }

    super::impl_mask_subreddits!(flairs => posts);

    fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<()> {
        if subreddit.is_moderator(post.author().as_str())? {
            return Ok(());
        }

        let now = Utc::now();

        let utc = into_timestamp(post.created_utc());
        let diff = now - utc;
        if diff.abs().num_seconds() < 30 {
            // delay to ignore any posts immediately removed by AutoMod.
            return Ok(());
        }

        let Some(flairs) = config.map(|cfg| &cfg.flairs) else {
            return Ok(());
        };

        if let Some(data) = client.flair_cache.inner.get(post.name()) {
            if data.delay_until > now {
                return Ok(());
            }
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
                    .any(|f| f == author_flair.as_str())
                {
                    continue;
                }
            }

            if title_lowercase.contains(flair.title_must_include.as_str()) {
                continue;
            }

            // At this point, the post uses the flair, the author is not exempt, and the title is not well-formed.
            // Change the flair accordingly.

            client
                .flair_cache
                .inner
                .entry(post.name().clone())
                .and_modify(|e| e.increment())
                .or_insert_with(PostFlairData::default);

            if !client.dry_run {
                post.select_flair(&flair.change_to)?;
                if let Some(webhook) = client.webhook.as_mut() {
                    webhook.send(&create_change_flair_message(post, &flair.change_to))?;
                }
                break;
            }
        }

        Ok(())
    }
}
