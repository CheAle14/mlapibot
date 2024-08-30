use std::collections::HashMap;

use serde::Deserialize;

use crate::{utils::LowercaseString, SubredditStatusConfig};

#[derive(Debug, Deserialize)]
pub struct SubredditsConfig(HashMap<LowercaseString, SubredditConfig>);

impl SubredditsConfig {
    pub fn get(&self, subreddit: &LowercaseString) -> Option<&SubredditConfig> {
        self.0.get(subreddit)
    }

    pub fn get_status(&self, subreddit: &LowercaseString) -> Option<&SubredditStatusConfig> {
        self.0.get(subreddit).and_then(|c| c.status.as_ref())
    }

    pub fn get_moderate(&self, subreddit: &LowercaseString) -> Option<&SubredditModerateConfig> {
        self.0.get(subreddit).and_then(|c| c.moderate.as_ref())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<LowercaseString, SubredditConfig> {
        self.0.keys()
    }
}

#[derive(Debug, Deserialize)]
pub struct SubredditConfig {
    pub status: Option<SubredditStatusConfig>,
    pub moderate: Option<SubredditModerateConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SubredditModerateConfig {
    pub removal_reason: String,
}
