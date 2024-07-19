use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use roux::{submission::SubmissionData, util::FeedOption, ThingId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SeenData {
    seen_id: ThingId,
    seen_time: DateTime<Utc>,
}

pub struct SeenTracker {
    seen_file: PathBuf,
    seen_data: Option<SeenData>,
}

fn into_timestamp(utc: f64) -> DateTime<Utc> {
    (Utc).timestamp_millis_opt((utc * 1000.0) as i64).unwrap()
}

impl SeenTracker {
    pub fn new(seen_file: PathBuf) -> Self {
        let seen_data = std::fs::read_to_string(&seen_file)
            .ok()
            .map(|s| serde_json::from_str(&s).unwrap());

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

    pub fn filter_seen(&self, mut iter: Vec<SubmissionData>) -> Vec<SubmissionData> {
        if let Some(seen) = &self.seen_data {
            iter.retain(|s| {
                let utc = into_timestamp(s.created_utc);
                utc > seen.seen_time
            });
            iter
        } else {
            iter
        }
    }

    pub fn set_seen(&mut self, id: &ThingId, timestamp: f64) {
        let timestamp = into_timestamp(timestamp);
        if let Some(existing) = &self.seen_data {
            if id.full() == existing.seen_id.full() || existing.seen_time >= timestamp {
                return;
            }
        }
        self.seen_data = Some(SeenData {
            seen_id: id.to_owned(),
            seen_time: timestamp,
        });
        let mut file = std::fs::File::create(&self.seen_file).expect("can open file");
        serde_json::to_writer(&mut file, &self.seen_data).expect("can write seen data");
    }
}
