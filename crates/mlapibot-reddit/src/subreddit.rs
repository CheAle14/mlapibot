use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use anyhow::Context;
use mlapibot_datastore::MlapiDb;
use roux::{
    api::{moderator::ModeratorData, subreddit::RemovalReason},
    util::{FeedOption, RouxError},
};
use statuspage::StatusClient;

use mlapibot_common::{Cached, LowercaseString};

use crate::config::SubredditStatusConfig;

use super::{
    RouxClient, Submission,
    status_tracker::{CachedIncidentSubmissions, StatusTracker},
};

pub type RouxSubreddit = roux::client::Subreddit<super::RouxClient>;

type SubCached<T> = Cached<T, RouxSubreddit, RouxError>;

pub struct Subreddit {
    data: RouxSubreddit,
    status: StatusTracker,
    lower: LowercaseString,
    moderators: SubCached<Vec<ModeratorData>>,
    removal_reasons: SubCached<HashMap<String, RemovalReason>>,
    // whether we are only using this subreddit to send status info
    pub status_only: bool,
}

fn minimal_urldecode(text: &mut String) {
    const GT: &str = "&gt;";

    while let Some(idx) = text.find(GT) {
        text.replace_range(idx..(idx + GT.len()), ">");
    }
}

fn fetch_removal_reasons(ctx: &RouxSubreddit) -> Result<HashMap<String, RemovalReason>, RouxError> {
    let mut map = ctx.list_removal_reasons()?.data;

    for value in map.values_mut() {
        minimal_urldecode(&mut value.message);
    }

    Ok(map)
}

impl Subreddit {
    pub fn new(
        scratch_dir: &Path,
        status_only: bool,
        data: RouxSubreddit,
        name: LowercaseString,
    ) -> anyhow::Result<Self> {
        let file = scratch_dir.join(format!("r_{}_last.json", data.name));
        let status = StatusTracker::new(scratch_dir.join(format!("r_{}_status.json", data.name)));

        let removal_reasons =
            Cached::new(Duration::from_secs(60 * 60), &data, fetch_removal_reasons)
                .context("init cache removal reasons")?;

        let moderators = Cached::new(Duration::from_secs(15 * 60), &data, |ctx| {
            ctx.moderators().map(|d| d.data.children)
        })
        .context("init moderators")?;

        Ok(Self {
            data,
            status,
            lower: name,
            status_only,
            removal_reasons,
            moderators: moderators,
        })
    }

    pub fn is_moderator(&mut self, username: &str) -> Result<bool, RouxError> {
        self.moderators
            .data(&self.data)
            .map(|ls| ls.iter().any(|m| m.name == username))
    }

    pub fn name(&self) -> &LowercaseString {
        &self.lower
    }

    pub fn update_status(
        &mut self,
        reddit: &RouxClient,
        status: &StatusClient,
        cached: &mut CachedIncidentSubmissions,
        is_summary: bool,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<()> {
        let mut unseen = HashSet::new();
        for id in self.status.map.posts.keys() {
            unseen.insert(id.clone());
        }

        for incident in &cached.incidents {
            let update = incident
                .updated_at
                .unwrap_or_else(|| incident.created_at)
                .to_utc();

            if self.status.is_tracking(incident.id.as_str()) {
                unseen.remove(&incident.id);
                if self.status.needs_update(incident) {
                    let cached =
                        CachedIncidentSubmissions::get_submission(&mut cached.cache, incident)?;

                    self.status.update(reddit, &incident.id, update, cached)?;
                }
            } else if incident.impact >= config.min_impact {
                unseen.remove(&incident.id);

                let cached =
                    CachedIncidentSubmissions::get_submission(&mut cached.cache, incident)?;

                self.status.add(
                    incident.id.as_str(),
                    update,
                    reddit,
                    &self.data,
                    config.flair_id.as_ref().map(|s| s.as_str()),
                    cached,
                )?;
            }
        }

        if !is_summary {
            // from a webhook, so it is expected that other incidents are missing
            return Ok(());
        }

        for unseen in unseen {
            let incident = status.get_incident(&unseen)?;

            let update = incident
                .updated_at
                .unwrap_or_else(|| incident.created_at)
                .to_utc();

            CachedIncidentSubmissions::add(&mut cached.cache, &incident)?;
            let cached = CachedIncidentSubmissions::get_submission(&mut cached.cache, &incident)?;
            self.status.update(reddit, &incident.id, update, cached)?;
            self.status.potentially_remove(&incident.id)?;
        }

        Ok(())
    }

    pub fn newest_unseen(&mut self) -> anyhow::Result<Vec<Submission>> {
        let options = FeedOption::new().limit(25);

        let data = self.data.latest(Some(options))?;
        let mut children = data.children;
        children.reverse();

        Ok(children)
    }

    pub fn get_removal_reason(&mut self, id: &str) -> Result<Option<&RemovalReason>, RouxError> {
        self.removal_reasons.data(&self.data).map(|map| map.get(id))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test_removal_decode() {
        const BEFORE: &str = "&gt; hello &gt;&gt;&gt; more text &gt;here";

        const AFTER: &str = "> hello >>> more text >here";

        let mut text = String::from(BEFORE);
        super::minimal_urldecode(&mut text);

        assert_eq!(text, AFTER);
    }
}
