use std::collections::HashSet;

use roux::builders::submission::SubmissionSubmitBuilder;
use statuspage::{incident::IncidentImpact, StatusClient};

use crate::RedditInfo;

use super::{
    seen_tracker::SeenTracker,
    status_tracker::{CachedSummary, StatusTracker},
};

pub struct Subreddit {
    data: roux::Subreddit,
    seen: SeenTracker,
    status: StatusTracker,
    // whether we are only using this subreddit to send status info
    pub status_only: bool,
}

impl Subreddit {
    pub fn new(args: &RedditInfo, data: roux::Subreddit) -> Self {
        let file = args.scratch_dir.join(format!("r_{}_last.txt", data.name));
        let seen = SeenTracker::new(file);
        let status = StatusTracker::new(
            args.scratch_dir
                .join(format!("r_{}_status.json", data.name)),
        );
        let status_only = args.subreddits.iter().find(|&s| s == &data.name).is_none();
        Self {
            data,
            seen,
            status,
            status_only,
        }
    }

    pub fn name(&self) -> &str {
        &self.data.name
    }

    pub fn update_status(
        &mut self,
        reddit: &roux::Me,
        status: &StatusClient,
        summary: &mut CachedSummary,
        level: &IncidentImpact,
    ) -> anyhow::Result<()> {
        let mut unseen = HashSet::new();
        for id in self.status.map.posts.keys() {
            unseen.insert(id.clone());
        }

        for incident in &summary.summary.incidents {
            if &incident.impact < level {
                continue;
            }
            unseen.remove(&incident.id);
            if self.status.is_tracking(incident.id.as_str()) {
                if self.status.needs_update(incident) {
                    let cached = CachedSummary::get_submission(&mut summary.cache, incident)?;
                    let text = match &cached.kind {
                        roux::builders::submission::SubmissionSubmitKind::SelfText { text } => {
                            text.as_str()
                        }
                        _ => unreachable!("we create this as a text post"),
                    };

                    self.status.update(reddit, &incident.id, text)?;
                }
            } else {
                let cached = CachedSummary::get_submission(&mut summary.cache, incident)?;
                self.status
                    .add(incident.id.as_str(), reddit, &self.data, cached)?;
            }
        }

        for unseen in unseen {
            let incident = status.get_incident(&unseen)?;
            CachedSummary::add(&mut summary.cache, &incident)?;
            let cached = CachedSummary::get_submission(&mut summary.cache, &incident)?;
            let text = match &cached.kind {
                roux::builders::submission::SubmissionSubmitKind::SelfText { text } => {
                    text.as_str()
                }
                _ => unreachable!("we create this as a text post"),
            };
            self.status.update(reddit, &incident.id, text)?;
            self.status.remove(&incident.id)?;
        }

        // TODO: handle incidents that are now resolved, and thus don't appear in the summary

        Ok(())
    }

    pub fn newest_unseen(&mut self) -> anyhow::Result<Vec<roux::submission::SubmissionData>> {
        let options = self.seen.get_options();
        let data = self.data.latest(25, options)?;
        if let Some(latest) = data.data.children.first() {
            self.seen.set_seen(latest.data.name.full());
        }
        let things: Vec<_> = data.data.children.into_iter().map(|d| d.data).collect();

        Ok(things)
    }
}
