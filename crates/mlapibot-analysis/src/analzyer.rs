use mlapibot_common::Detection;
use serde::{Deserialize, Deserializer};

use crate::{Context, matcher::Matcher};

fn default_template() -> String {
    String::from("default.md")
}

fn default_true() -> bool {
    true
}

// Any value that is present is considered Some value, including null.
// https://github.com/serde-rs/serde/issues/984#issuecomment-314143738
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
struct RawTemplateName {
    #[serde(default, deserialize_with = "deserialize_some")]
    template: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(from = "RawTemplateName")]
pub struct TemplateName(Option<String>);

impl From<RawTemplateName> for TemplateName {
    fn from(value: RawTemplateName) -> Self {
        match value.template {
            Some(None) => Self(None),
            None => Self(Some(default_template())),
            Some(Some(text)) => Self(Some(format!("{text}.md"))),
        }
    }
}

impl TemplateName {
    pub fn name(&self) -> Option<&str> {
        self.0.as_ref().map(|s| s.as_str())
    }
}

#[derive(Deserialize, Debug)]
pub struct Analyzer {
    pub name: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub report: bool,
    #[serde(default)]
    pub remove: bool,
    #[serde(default = "default_true")]
    pub ignore_self_posts: bool,
    #[serde(flatten)]
    pub template: TemplateName,
    blacklist: Option<crate::matcher::MatcherKind>,
    #[serde(flatten)]
    kind: AnalyzerKind,
}

impl Analyzer {
    pub fn analyze(&self, context: &Context) -> crate::error::Result<Option<Detection>> {
        let result = self.kind.analyze(context)?;
        if let Some(result) = result {
            if let Some(blacklist) = &self.blacklist {
                if blacklist.any_matches(context) {
                    return Ok(None);
                }
            }

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

mod function;
mod pattern;
mod string;

pub use function::*;
pub use pattern::*;
pub use string::*;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AnalyzerKind {
    #[serde(rename = "function")]
    Function(function::FuncAnalyzer),
    #[serde(rename = "img")]
    Pattern(pattern::PatternAnalyzer),
    #[serde(untagged)]
    Text(string::StrAnalzyer),
}

impl AnalyzerKind {
    fn analyze(&self, context: &Context) -> crate::error::Result<Option<Detection>> {
        match self {
            AnalyzerKind::Text(v) => v.analyze(context),
            AnalyzerKind::Function(v) => v.analyze(context),
            AnalyzerKind::Pattern(v) => v.analyze(context),
        }
    }
}
