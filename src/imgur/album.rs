use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Album {
    pub id: String,
    #[serde(rename = "deletehash")]
    pub delete_hash: String,
}

#[derive(Serialize)]
pub struct AlbumBuilder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AlbumBuilder {
    pub fn builder() -> Self {
        Self {
            title: None,
            description: None,
        }
    }

    pub fn title(mut self, title: impl AsRef<str>) -> Self {
        self.title = Some(title.as_ref().to_string());
        self
    }

    pub fn with_title(&mut self, title: impl AsRef<str>) -> &mut Self {
        self.title = Some(title.as_ref().to_string());
        self
    }
}
