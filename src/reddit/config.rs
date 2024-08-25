use std::collections::HashMap;

use serde::Deserialize;

use crate::SubredditStatusConfig;

#[derive(Deserialize)]
pub struct SubredditsConfig(HashMap<String, SubredditConfig>);

impl SubredditsConfig {
    pub fn get(&self, subreddit: &str) -> Option<&SubredditConfig> {
        self.0.get(subreddit)
    }

    pub fn get_status(&self, subreddit: &str) -> Option<&SubredditStatusConfig> {
        self.0.get(subreddit).and_then(|c| c.status.as_ref())
    }

    pub fn get_moderate(&self, subreddit: &str) -> Option<&SubredditModerateConfig> {
        self.0.get(subreddit).and_then(|c| c.moderate.as_ref())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<String, SubredditConfig> {
        self.0.keys()
    }
}

#[derive(Deserialize)]
pub struct SubredditConfig {
    pub status: Option<SubredditStatusConfig>,
    pub moderate: Option<SubredditModerateConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SubredditModerateConfig {
    pub removal_reason: String,
}
