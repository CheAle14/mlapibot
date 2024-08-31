use std::collections::HashSet;

use roux::util::FeedOption;
use statuspage::{incident::IncidentImpact, StatusClient};

use crate::{utils::LowercaseString, RedditInfo, SubredditStatusConfig};

use super::{
    seen_tracker::SeenTracker,
    status_tracker::{CachedIncidentSubmissions, StatusTracker},
    RouxClient, Submission,
};

pub type RouxSubreddit = roux::client::Subreddit<super::RouxClient>;

pub struct Subreddit {
    data: RouxSubreddit,
    seen: SeenTracker,
    status: StatusTracker,
    lower: LowercaseString,
    // whether we are only using this subreddit to send status info
    pub status_only: bool,
}

impl Subreddit {
    pub fn new(args: &RedditInfo, data: RouxSubreddit, name: LowercaseString) -> Self {
        let file = args.scratch_dir.join(format!("r_{}_last.json", data.name));
        let seen = SeenTracker::new(file);
        let status = StatusTracker::new(
            args.scratch_dir
                .join(format!("r_{}_status.json", data.name)),
        );
        let status_only = args.subreddits.iter().find(|&s| s == &name).is_none();
        Self {
            data,
            seen,
            status,
            lower: name,
            status_only,
        }
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
                    let text = match &cached.kind {
                        roux::builders::submission::SubmissionSubmitKind::SelfText { text } => {
                            text.as_str()
                        }
                        _ => unreachable!("we create this as a text post"),
                    };

                    self.status.update(reddit, &incident.id, update, text)?;
                }
            } else if incident.impact >= config.min_impact {
                unseen.remove(&incident.id);
                let cached =
                    CachedIncidentSubmissions::get_submission(&mut cached.cache, incident)?;

                if let Some(flair) = config.flair_id.as_ref() {
                    let cloned = cached.clone().with_flair_id(flair.as_str());

                    self.status
                        .add(incident.id.as_str(), update, reddit, &self.data, &cloned)?;
                } else {
                    self.status
                        .add(incident.id.as_str(), update, reddit, &self.data, cached)?;
                }
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
            let text = match &cached.kind {
                roux::builders::submission::SubmissionSubmitKind::SelfText { text } => {
                    text.as_str()
                }
                _ => unreachable!("we create this as a text post"),
            };
            self.status.update(reddit, &incident.id, update, text)?;
            self.status.remove(&incident.id)?;
        }

        Ok(())
    }

    pub fn newest_unseen(&mut self) -> anyhow::Result<Vec<Submission>> {
        let options = self
            .seen
            .get_options()
            .unwrap_or_else(|| FeedOption::new())
            .limit(25);

        let data = self.data.latest(Some(options))?;
        let mut children = self.seen.filter_seen(data.children);

        children.reverse();
        println!("Saw {} posts in latest", children.len());

        Ok(children)
    }

    pub fn set_seen(&mut self, post: &Submission) {
        self.seen.set_seen(&post.name(), post.created_utc());
    }
}
