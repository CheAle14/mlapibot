use serde::Deserialize;

use crate::ocr::image::ImageSource;

#[derive(Debug, Deserialize)]
pub struct Image {
    pub id: String,
    #[serde(rename = "deletehash")]
    pub delete_hash: String,
}

pub struct ImageBuilder<'img> {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: &'img ImageSource,
}

impl<'img> ImageBuilder<'img> {
    pub fn builder(image: &'img ImageSource) -> Self {
        ImageBuilder {
            title: None,
            description: None,
            image,
        }
    }

    pub fn description(mut self, description: impl AsRef<str>) -> Self {
        self.description = Some(description.as_ref().to_string());
        self
    }
}
