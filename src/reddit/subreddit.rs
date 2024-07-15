use std::path::PathBuf;

use roux::util::FeedOption;

use crate::RedditInfo;

use super::seen_tracker::SeenTracker;

pub struct Subreddit {
    data: roux::Subreddit,
    seen: SeenTracker,
}

impl Subreddit {
    pub fn new(args: &RedditInfo, data: roux::Subreddit) -> Self {
        let file = args.data_dir.join(format!("r_{}_last.txt", data.name));
        let seen = SeenTracker::new(file);
        Self { data, seen }
    }

    pub fn newest_unseen(&mut self) -> anyhow::Result<Vec<roux::submission::SubmissionData>> {
        let options = self.seen.get_options();
        let data = self.data.latest(25, options)?;
        if let Some(latest) = data.data.children.first() {
            self.seen.set_seen(&latest.data.name);
        }
        let things: Vec<_> = data.data.children.into_iter().map(|d| d.data).collect();

        Ok(things)
    }
}
