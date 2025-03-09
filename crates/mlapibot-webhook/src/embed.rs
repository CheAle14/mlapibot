#[derive(Debug, serde::Serialize)]
pub struct MessageEmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

impl MessageEmbedAuthor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: None,
            icon_url: None,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MessageEmbed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<MessageEmbedAuthor>,
}

impl MessageEmbed {
    pub fn builder() -> Self {
        Self {
            title: None,
            description: None,
            url: None,
            author: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.with_title(title);
        self
    }

    pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.with_description(description);
        self
    }

    pub fn with_description(&mut self, description: impl Into<String>) -> &mut Self {
        self.description = Some(description.into());
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.with_url(url);
        self
    }

    pub fn with_url(&mut self, url: impl Into<String>) -> &mut Self {
        self.url = Some(url.into());
        self
    }

    pub fn author(mut self, author: MessageEmbedAuthor) -> Self {
        self.with_author(author);
        self
    }

    pub fn with_author(&mut self, author: MessageEmbedAuthor) -> &mut Self {
        self.author = Some(author);
        self
    }
}
