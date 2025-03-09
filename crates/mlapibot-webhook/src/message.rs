use crate::embed::MessageEmbed;

#[derive(Debug, serde::Serialize)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<MessageEmbed>,
}

impl Message {
    pub fn builder() -> Self {
        Self {
            content: None,
            embeds: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.with_content(content);
        self
    }

    #[inline(always)]
    pub fn with_content(&mut self, content: impl Into<String>) -> &mut Self {
        self.content = Some(content.into());
        self
    }

    #[inline(always)]
    pub fn embed(mut self, embed: MessageEmbed) -> Self {
        self.with_embed(embed);
        self
    }

    #[inline(always)]
    pub fn with_embed(&mut self, embed: MessageEmbed) -> &mut Self {
        self.embeds.push(embed);
        self
    }
}
