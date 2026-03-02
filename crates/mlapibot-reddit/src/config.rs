use roux::{builders::submission::SubmissionSubmitBuilder, client::SelectFlairData};

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
