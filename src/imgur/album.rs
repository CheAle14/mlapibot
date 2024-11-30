use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletehashes: Option<Vec<String>>,
}

impl AlbumBuilder {
    pub fn builder() -> Self {
        Self {
            title: None,
            description: None,
            deletehashes: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    pub fn delete_hashes<'a>(mut self, names: impl Iterator<Item = &'a str>) -> Self {
        let v: Vec<String> = names.map(String::from).collect();
        self.deletehashes = Some(v);
        self
    }
}
