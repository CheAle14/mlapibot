use serde::Serializer;

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
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_int"
    )]
    pub color: Option<[u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<MessageEmbedFooter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<MessageEmbedField>,
}

fn serialize_int<S: Serializer>(value: &Option<[u8; 3]>, s: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => {
            let int = u32::from_be_bytes([0, value[0], value[1], value[2]]);
            s.serialize_u32(int)
        }
        None => s.serialize_none(),
    }
}

impl MessageEmbed {
    pub fn builder() -> Self {
        Self {
            title: None,
            description: None,
            url: None,
            author: None,
            color: None,
            footer: None,
            fields: Vec::new(),
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

    pub fn color(mut self, red: u8, green: u8, blue: u8) -> Self {
        self.with_color(red, green, blue);
        self
    }

    pub fn with_color(&mut self, red: u8, green: u8, blue: u8) -> &mut Self {
        self.color = Some([red, green, blue]);
        self
    }

    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_field(name, value);
        self
    }

    pub fn with_field(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.fields.push(MessageEmbedField {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    pub fn footer(mut self, footer: MessageEmbedFooter) -> Self {
        self.with_footer(footer);
        self
    }

    pub fn with_footer(&mut self, footer: MessageEmbedFooter) -> &mut Self {
        self.footer = Some(footer);
        self
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MessageEmbedField {
    name: String,
    value: String,
}

#[derive(Debug, serde::Serialize)]
pub struct MessageEmbedFooter {
    text: String,
    icon_url: Option<String>,
}

impl MessageEmbedFooter {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon_url: None,
        }
    }

    pub fn icon_url(mut self, url: impl Into<String>) -> Self {
        self.with_icon_url(url);
        self
    }

    pub fn with_icon_url(&mut self, url: impl Into<String>) -> &mut Self {
        self.icon_url = Some(url.into());
        self
    }
}
