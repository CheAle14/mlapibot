use std::{collections::VecDeque, path::PathBuf, time::Duration};

use chrono::{DateTime, TimeZone, Utc};
use roux::{api::ThingId, util::FeedOption};
use serde::{Deserialize, Serialize};

use super::Submission;

#[derive(Debug, Serialize, Deserialize)]
struct SeenData {
    seen_time: DateTime<Utc>,
    #[serde(default)]
    seen_ids: VecDeque<ThingId>,
}

pub struct SeenTracker {
    seen_file: PathBuf,
    seen_data: SeenData,
}

fn into_timestamp(utc: f64) -> DateTime<Utc> {
    (Utc).timestamp_millis_opt((utc * 1000.0) as i64).unwrap()
}

impl SeenTracker {
    pub fn new(seen_file: PathBuf) -> Self {
        let seen_data = std::fs::read_to_string(&seen_file)
            .ok()
            .map(|s| serde_json::from_str(&s).unwrap())
            .unwrap_or_else(|| SeenData {
                seen_time: Utc::now() - Duration::from_secs(3600 * 24 * 7 * 4),
                seen_ids: VecDeque::with_capacity(100),
            });

        Self {
            seen_file,
            seen_data,
        }
    }

    pub fn get_options(&self) -> Option<FeedOption> {
        // unfortunately reddit seems to return nothing if you use `before` or `after` with a post that
        // has been removed, so we'll just keep fetching the latest posts and filter out anything not new
        // we could technically work around this by remembering the second-newest post, as the children list
        // should then have exactly one item (the actual latest one) if there are no new posts, so if it is zero
        // we know it has been removed and can then re-fetch latest (potentially walking all the way back
        // to the timestamp of the removed post)
        None
    }

    pub fn is_seen(&self, post: &Submission) -> bool {
        let utc = into_timestamp(post.created_utc());
        self.seen_data.seen_time >= utc || self.seen_data.seen_ids.contains(post.name())
    }

    pub fn set_seen(&mut self, id: &ThingId, _timestamp: f64) {
        // seen_time represents the last post seen before
        // switching over to the vecdeque, so don't update it
        //
        //      self.seen_data.seen_time = self.seen_data.seen_time.max(into_timestamp(timestamp));

        self.seen_data.seen_ids.push_back(id.clone());

        if self.seen_data.seen_ids.len() > 100 {
            self.seen_data.seen_ids.pop_front();
        }

        let mut file = std::fs::File::create(&self.seen_file).expect("can open file");
        serde_json::to_writer(&mut file, &self.seen_data).expect("can write seen data");
    }
}
