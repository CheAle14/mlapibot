use std::fmt::Debug;

use serde::Serialize;

use crate::reddit::{CreatedCommentWithLinkInfo, RedditMessage, Submission};
use crate::utils::clamp;

#[derive(Debug, Serialize)]
pub struct MessageEmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

impl MessageEmbedAuthor {
    pub fn new(name: impl AsRef<str>) -> Self {
        let name = name.as_ref();
        Self {
            name: name.to_string(),
            url: None,
            icon_url: None,
        }
    }
}

#[derive(Debug, Serialize)]
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

    pub fn title(mut self, title: impl AsRef<str>) -> Self {
        self.with_title(title);
        self
    }

    pub fn with_title(&mut self, title: impl AsRef<str>) -> &mut Self {
        self.title = Some(title.as_ref().to_string());
        self
    }

    pub fn description(mut self, description: impl AsRef<str>) -> Self {
        self.with_description(description);
        self
    }

    pub fn with_description(&mut self, description: impl AsRef<str>) -> &mut Self {
        self.description = Some(description.as_ref().to_string());
        self
    }

    pub fn url(mut self, url: impl AsRef<str>) -> Self {
        self.with_url(url);
        self
    }

    pub fn with_url(&mut self, url: impl AsRef<str>) -> &mut Self {
        self.url = Some(url.as_ref().to_string());
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

#[derive(Debug, Serialize)]
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
    pub fn content(mut self, content: impl AsRef<str>) -> Self {
        self.with_content(content);
        self
    }

    #[inline(always)]
    pub fn with_content(&mut self, content: impl AsRef<str>) -> &mut Self {
        self.content = Some(content.as_ref().to_string());
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

pub struct WebhookClient {
    client: reqwest::blocking::Client,
    url: String,
}

impl WebhookClient {
    pub fn new(url: impl AsRef<str>) -> anyhow::Result<Self> {
        let s = url.as_ref();
        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent("DiscordBot (https://github.com/CheAle14/mlapibot, rust-1)")
            .build()?;
        Ok(Self {
            url: s.to_string(),
            client,
        })
    }

    pub fn send(&mut self, message: &Message) -> anyhow::Result<()> {
        let response = self.client.post(&self.url).json(message).send()?;
        match response.error_for_status_ref() {
            Ok(_) => Ok(()),
            Err(err) => {
                let body = response.text()?;
                eprintln!("Response body: {body}");
                Err(err.into())
            }
        }
    }
}

trait LinkExt {
    fn with_reddit_link(&mut self, maybe_full_url: &str) -> &mut Self;
    fn reddit_link(mut self, maybe_full_url: &str) -> Self
    where
        Self: Sized,
    {
        self.with_reddit_link(maybe_full_url);
        self
    }
}

impl LinkExt for MessageEmbed {
    fn with_reddit_link(&mut self, maybe_full_url: &str) -> &mut Self {
        if maybe_full_url.starts_with("http") {
            self.with_url(maybe_full_url)
        } else {
            let full = format!("https://reddit.com{maybe_full_url}");
            self.with_url(full)
        }
    }
}

pub fn create_detection_message(
    submission: &Submission,
    detection: &crate::analysis::Detection,
    analyzer: &crate::analysis::Analyzer,
    imgur_link: Option<String>,
) -> Message {
    let mut embed = MessageEmbed::builder();
    embed
        .with_title(&submission.title())
        .with_description(format!(
            "{}: {:.2}%{}",
            analyzer.name,
            detection.best_score() * 100.0,
            imgur_link
                .map(|s| format!("\r\n\r\n[OCR]({s})"))
                .unwrap_or("".into())
        ))
        .with_reddit_link(submission.permalink())
        .with_author(MessageEmbedAuthor::new(&submission.author()));

    let mut message = Message::builder();
    message.with_embed(embed);
    message
}

pub fn create_inbox_message(message: &RedditMessage) -> Message {
    let subject = clamp(&message.subject(), 128);
    let description = clamp(&message.body(), 4096);
    let author = if let Some(author) = &message.author() {
        MessageEmbedAuthor::new(author)
    } else {
        MessageEmbedAuthor::new("no author")
    };

    let mut embed = MessageEmbed::builder()
        .title(format!("Inbox: {}", subject))
        .description(description)
        .author(author);

    let ctx = message.context();
    embed.with_reddit_link(ctx);
    Message::builder().embed(embed)
}

pub fn create_error_processing_post(post: &Submission) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Error occured processing post")
            .description(format!(
                "Post [`{}`](https://reddit.com{}) by /u/{} caused an error",
                post.title(),
                post.permalink(),
                post.author()
            ))
            .reddit_link(post.permalink()),
    )
}
pub fn create_error_processing_message(message: &RedditMessage) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Error occured processing message")
            .description(format!(
                "From /u/{} subject:\r\n> {}",
                message.author().clone().unwrap_or_default(),
                message.subject(),
            )),
    )
}
pub fn create_deleted_downvoted_comment(comment: &CreatedCommentWithLinkInfo) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Removed downvoted post")
            .description(format!("For {}", comment.link_title()))
            .reddit_link(comment.permalink()),
    )
}

fn get_error_embed(err: impl Debug) -> MessageEmbed {
    let text = format!("{err:?}");
    let clamped = clamp(&text, 4096 - (3 + 3 + 2 + 2));
    let actual = format!("```\r\n{clamped}\r\n```");
    MessageEmbed::builder().description(&actual)
}

pub fn create_generic_error_message(content: impl AsRef<str>, err: impl Debug) -> Message {
    let content = content.as_ref();
    eprintln!("Error {content}: {err:?}");
    Message::builder()
        .content(content)
        .embed(get_error_embed(err))
}

pub fn create_multiple_error_message(
    content: impl AsRef<str>,
    errs: Vec<anyhow::Error>,
) -> Message {
    let content = content.as_ref();
    eprintln!("Multiple error: {content}");
    let mut message = Message::builder().content(content);

    for error in errs {
        eprintln!("  - {error:?}");
        message.with_embed(get_error_embed(error));
    }

    message
}