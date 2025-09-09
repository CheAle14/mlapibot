use mlapibot_analysis::analzyer::Analyzer;
use mlapibot_common::Detection;
use mlapibot_webhook::{LinkExt, Message, MessageEmbed, MessageEmbedAuthor};

use crate::{CreatedCommentWithLinkInfo, RedditMessage, Submission, utils::clamp};

pub fn create_detection_message(
    submission: &Submission,
    detection: &Detection,
    analyzer: &Analyzer,
    imgur_link: Option<String>,
) -> Message {
    let mut embed = MessageEmbed::builder();
    embed
        .with_title(submission.title())
        .with_description(format!(
            "{}: {:.2}%{}",
            analyzer.name,
            detection.best_score() * 100.0,
            imgur_link
                .map(|s| format!("\r\n\r\n[OCR]({s})"))
                .unwrap_or("".into())
        ))
        .with_reddit_link(submission.permalink())
        .with_author(MessageEmbedAuthor::new(submission.author()));

    let mut message = Message::builder();
    message.with_embed(embed);
    message
}

pub fn create_change_flair_message(submission: &Submission, now_flair: &str) -> Message {
    let embed = MessageEmbed::builder()
        .title("Flair updated")
        .description(format!(
            "Was `{:?}` now `{now_flair}`",
            submission.link_flair_template_id()
        ))
        .reddit_link(submission.permalink())
        .author(MessageEmbedAuthor::new(submission.author()));

    Message::builder().embed(embed)
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
pub fn create_error_processing_message(author: &str, subject: &str) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Error occured processing message")
            .description(format!("From /u/{author} subject:\r\n>>> {subject}",)),
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
pub fn create_moderator_downvoted_comment(comment: &CreatedCommentWithLinkInfo) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Distinguished comment downvoted")
            .description(format!("For {}", comment.link_title()))
            .reddit_link(comment.permalink()),
    )
}
