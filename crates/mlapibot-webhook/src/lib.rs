mod client;
mod embed;
mod error;
mod link_ext;
mod message;

pub use client::WebhookClient;
pub use embed::*;
pub use error::*;
pub use link_ext::*;
pub use message::*;

fn clamp(text: &str, len: usize) -> &str {
    if text.len() > len { &text[..len] } else { text }
}

fn get_error_embed(err: impl Into<String>) -> MessageEmbed {
    let text = err.into();
    let clamped = clamp(&text, 4096 - (3 + 3 + 2 + 2));
    let actual = format!("```\r\n{clamped}\r\n```");
    MessageEmbed::builder().description(&actual)
}

pub fn create_generic_error_message(content: impl Into<String>, err: impl Into<String>) -> Message {
    let content: String = content.into();
    let errstr = err.into();
    eprintln!("Error {content}: {}", errstr);
    Message::builder()
        .content(content)
        .embed(get_error_embed(errstr))
}

pub fn create_multiple_error_message(
    content: impl Into<String>,
    errs: Vec<impl ToString>,
) -> Message {
    let content: String = content.into();
    eprintln!("Multiple error: {content}");
    let mut message = Message::builder().content(content);

    for error in errs {
        let error: String = error.to_string();
        eprintln!("  - {error}");
        message.with_embed(get_error_embed(error));
    }

    message
}
