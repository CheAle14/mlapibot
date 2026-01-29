use mlapibot_webhook::{LinkExt, Message, MessageEmbed, MessageEmbedAuthor};
use roux::client::SelectFlairData;

use crate::{
    CreatedCommentWithLinkInfo, RedditMessage, Submission, config::PostFlairSetting, utils::clamp,
};

pub fn create_detection_message(
    submission: &Submission,
    module: &str,
    analyser: Option<&str>,
    is_debug: bool,
) -> Message {
    let mut embed = MessageEmbed::builder()
        .title(submission.title())
        .description(format!(
            "{module}: {}",
            analyser
                .map(|c| c.to_string())
                .unwrap_or_else(|| String::from("(no analyser)"))
        ))
        .reddit_link(submission.permalink())
        .author(MessageEmbedAuthor::new(submission.author()));

    if is_debug {
        embed.with_color(255, 0, 0);
    }

    Message::builder().embed(embed)
}

pub fn create_change_flair_message(
    submission: &Submission,
    now_flair: &PostFlairSetting,
) -> Message {
    let embed = MessageEmbed::builder()
        .title("Flair updated")
        .description(format!(
            "- Template: was `{:?}` now `{:?}`\n- Text: was `{:?}` now `{:?}`",
            submission.link_flair_template_id(),
            now_flair.template(),
            submission.link_flair_text(),
            now_flair.text()
        ))
        .reddit_link(submission.permalink())
        .author(MessageEmbedAuthor::new(submission.author()));

    Message::builder().embed(embed)
}

pub fn create_inbox_message(message: &RedditMessage) -> Message {
    let subject = clamp(&message.subject(), 128);
    let description = clamp(&message.body(), 4096);
    let author = MessageEmbedAuthor::new(message.author().unwrap_or("no author"));

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
