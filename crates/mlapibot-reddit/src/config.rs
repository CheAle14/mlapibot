use std::{borrow::Borrow, collections::HashMap};

use roux::{api::FlairId, builders::submission::SubmissionSubmitBuilder, client::SelectFlairData};
use serde::Deserialize;

use mlapibot_common::{LowercaseHashMap, LowercaseString};
use statuspage::incident::IncidentImpact;

use crate::client::module::post_flairs::SubredditFlairConfig;

#[derive(Debug, Deserialize)]
pub struct SubredditsConfig(LowercaseHashMap<SubredditConfig>);

impl SubredditsConfig {
    pub fn get<Q>(&self, subreddit: &Q) -> Option<&SubredditConfig>
    where
        Q: Borrow<str> + ?Sized,
    {
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

    pub fn keys(&self) -> impl Iterator<Item = &LowercaseString> {
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
    pub mods_can_stop: bool,

    #[serde(default)]
    pub comments_code: bool,
    #[serde(default)]
    pub comments_cdn: bool,
    #[serde(default)]
    pub comments_staff_reply: Option<SubredditStaffReplyConfig>,
    #[serde(default)]
    pub comments_complex: Vec<SubredditComplexCommentConfig>,
    #[serde(default)]
    pub related_titles: bool,
    #[serde(default)]
    pub ai_slop: Option<SubredditAiSlopConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SubredditModerateConfig {
    /// The removal reason ID to use for a particular analzyer
    pub removal_reasons: HashMap<String, String>,
    /// If the analzyer is not in the above map, the default reason to use.
    pub default_removal_reason: String,
}

/// When making a post to a subreddit, this describes the flair settings to be used.
///
/// Can be set by passing a string directly for the template ID, otherwise a struct
/// containing the text (and optional template ID)
#[derive(Debug, PartialEq)]
pub struct PostFlairSetting {
    // At least one must be specified.
    template: Option<String>,
    text: Option<String>,
}

impl PostFlairSetting {
    pub fn apply(&self, builder: SubmissionSubmitBuilder) -> SubmissionSubmitBuilder {
        let builder = match self.template {
            Some(ref id) => builder.with_flair_id(id),
            None => builder,
        };

        match self.text {
            Some(ref text) => builder.with_flair_text(text),
            None => builder,
        }
    }

    pub fn as_update(&self) -> SelectFlairData {
        SelectFlairData::new(self.template.clone(), self.text.clone())
    }

    pub fn template(&self) -> Option<&str> {
        self.template.as_ref().map(|v| v.as_str())
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_ref().map(|v| v.as_str())
    }
}

pub trait FlairSubBuilderExt: Sized {
    fn with_flair_setting(self, setting: &PostFlairSetting) -> Self;
}

impl FlairSubBuilderExt for SubmissionSubmitBuilder {
    fn with_flair_setting(self, setting: &PostFlairSetting) -> Self {
        setting.apply(self)
    }
}

impl<'de> serde::Deserialize<'de> for PostFlairSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visit;

        impl<'de> serde::de::Visitor<'de> for Visit {
            type Value = PostFlairSetting;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a flair template id, or a struct containing one and flair text to use")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(v.to_owned())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(PostFlairSetting {
                    template: Some(v),
                    text: None,
                })
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(v.to_owned())
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut template = None;
                let mut text = None;

                while let Some((k, v)) = map.next_entry::<String, String>()? {
                    match k.as_str() {
                        "template_id" | "template" => template = Some(v),
                        "text" => text = Some(v),
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &k,
                                &["template_id", "text"],
                            ));
                        }
                    }
                }

                Ok(PostFlairSetting { template, text })
            }
        }

        deserializer.deserialize_any(Visit)
    }
}

#[derive(Debug, Deserialize)]
pub struct SubredditStatusConfig {
    pub min_impact: statuspage::incident::IncidentImpact,
    pub flair: Option<SubredditStatusFlairConfig>,
    pub sticky: Option<StatusStickyConfig>,
    /// Whether it should distinguish the posts it makes.
    #[serde(default)]
    pub distinguish: bool,
}

#[derive(Debug, Deserialize)]
pub struct SubredditStatusFlairConfig {
    #[serde(alias = "incident")]
    pub minor: PostFlairSetting,
    pub major: Option<PostFlairSetting>,
    pub resolved: Option<PostFlairSetting>,
}

#[derive(Debug, Deserialize)]
pub struct SubredditStaffReplyConfig {
    /// Any user with this flair template ID is considered staff.
    pub staff_flair_id: String,
    /// If set, any user with this flair css class is also considered staff.
    pub staff_css_class: Option<String>,
    /// Any staff replies under a post with any of these texts in the title are ignored.
    #[serde(default)]
    pub ignore_post_title_contains: Vec<String>,
}

impl SubredditStaffReplyConfig {
    pub fn is_staff(&self, template_id: Option<&str>, css_class: Option<&str>) -> bool {
        template_id.is_some_and(|id| id == self.staff_flair_id)
            || self.staff_css_class.as_ref().map(|v| v.as_str()) == css_class
    }
}

#[derive(Debug, Deserialize)]
pub struct SubredditComplexCommentConfig {
    pub link_title: Vec<String>,
    pub comment: Vec<String>,
    #[serde(default)]
    pub ignore_flairs: Vec<String>,
    pub reason_id: String,
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

#[derive(Debug, Deserialize)]
pub struct SubredditAiSlopConfig {
    pub modmail_to: String,
}

#[derive(Clone, Deserialize)]
pub struct GlobalSettings {
    pub webhook_url: Option<String>,
    pub reddit: RedditSettings,
    pub imgur: Option<ImgurSettings>,
    pub github: Option<GithubSettings>,
}

#[derive(Clone, Deserialize)]
pub struct RedditSettings {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Deserialize)]
pub struct ImgurSettings {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Clone, Deserialize)]
pub struct GithubSettings {
    pub token: String,
}
