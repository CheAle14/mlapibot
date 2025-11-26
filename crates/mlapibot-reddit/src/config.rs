use std::collections::HashMap;

use roux::api::FlairId;
use serde::Deserialize;

use mlapibot_common::LowercaseString;
use statuspage::incident::IncidentImpact;

use crate::client::module::post_flairs::SubredditFlairConfig;

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

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SubredditConfig {
    #[serde(default)]
    pub scams: bool,
    pub status: Option<SubredditStatusConfig>,
    pub moderate: Option<SubredditModerateConfig>,
    #[serde(default)]
    pub flairs: SubredditFlairConfig,

    #[serde(default)]
    pub comments_code: bool,
    #[serde(default)]
    pub comments_cdn: bool,
    #[serde(default)]
    pub comments_staff_reply: Option<SubredditStaffReplyConfig>,
    #[serde(default)]
    pub related_titles: bool,
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
    pub sticky: Option<StatusStickyConfig>,
    /// Whether it should distinguish the posts it makes.
    #[serde(default)]
    pub distinguish: bool,
}

#[derive(Debug, Deserialize)]
pub struct SubredditStaffReplyConfig {
    /// Any user with this flair template ID is considered staff.
    pub staff_flair_id: String,
    /// Any staff replies under a post with any of these texts in the title are ignored.
    #[serde(default)]
    pub ignore_post_title_contains: Vec<String>,
}

fn default_comment_threshold() -> u64 {
    10
}

fn default_minor_delay() -> u32 {
    15
}

fn default_major_delay() -> u32 {
    180
}

#[derive(Debug, Deserialize)]
pub struct StatusStickyConfig {
    /// A different sticky post that we replace with the status sticky.
    /// New: now identitied by a flair ID, not post fullname.
    pub replace_sticky: Option<FlairId>,
    #[serde(default = "default_comment_threshold")]
    pub comment_threshold: u64,
    /// How long to wait after the incident resolves to unsticky (and restore the above)
    /// For posts with < `minor_comment_threshold` comments
    #[serde(default = "default_minor_delay")]
    pub delay_minor_mins: u32,
    /// How long to wait after the incident resolves to unsticky (and restore the above)
    /// For posts with >= `minor_comment_threshold` comments
    #[serde(default = "default_major_delay")]
    pub delay_major_mins: u32,
    /// If set, the minimum impact needed to sticky the post.
    /// If absent, any post sent to the subreddit is stickied.
    #[serde(default)]
    pub min_impact: Option<IncidentImpact>,
    /// If present, only sticky posts which affect the specified components
    #[serde(default)]
    pub only_for: Vec<String>,
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
