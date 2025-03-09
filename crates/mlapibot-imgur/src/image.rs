use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Image {
    pub id: String,
    #[serde(rename = "deletehash")]
    pub delete_hash: String,
}

pub struct ImageBuilder<'img> {
    pub title: Option<String>,
    pub description: Option<String>,
    pub path: &'img Path,
}

impl<'img> ImageBuilder<'img> {
    pub fn builder(path: &'img Path) -> Self {
        ImageBuilder {
            title: None,
            description: None,
            path,
        }
    }

    pub fn description(mut self, description: impl AsRef<str>) -> Self {
        self.description = Some(description.as_ref().to_string());
        self
    }
}
