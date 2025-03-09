use std::collections::HashMap;

use serde::Deserialize;

use mlapibot_common::LowercaseString;

use super::flairs::SubredditFlairConfig;

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
    #[serde(default)]
    pub flairs: SubredditFlairConfig,
}

#[derive(Debug, Deserialize)]
pub struct SubredditModerateConfig {
    /// The removal reason ID to use for a particular analzyer
    pub removal_reasons: HashMap<String, String>,
    /// If the analzyer is not in the above map, the default reason to use.
    pub default_removal_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SubredditStatusConfig {
    pub min_impact: statuspage::incident::IncidentImpact,
    pub flair_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct ImgurCredentials {
    pub imgur_client_id: String,
    pub imgur_client_secret: String,
}

#[derive(Clone, Deserialize)]
pub struct RedditCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub webhook_url: Option<String>,
    #[serde(flatten)]
    pub imgur_credentials: Option<ImgurCredentials>,
}
