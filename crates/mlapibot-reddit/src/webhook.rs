use mlapibot_webhook::{LinkExt, Message, MessageEmbed, MessageEmbedAuthor, MessageEmbedFooter};

use crate::{
    CreatedCommentWithLinkInfo, RedditMessage, Submission, config::PostFlairSetting, utils::clamp,
};

pub fn create_detection_message(
    submission: &Submission,
    module: &str,
    analyser: Option<&str>,
    additional_text: Option<&str>,
    is_debug: bool,
) -> Message {
    let mut embed = MessageEmbed::builder()
        .title(submission.title())
        .footer(MessageEmbedFooter::new(submission.subreddit()))
        .reddit_link(submission.permalink())
        .author(MessageEmbedAuthor::new(submission.author()))
        .field("Module", module);

    if let Some(analyser) = analyser {
        embed.with_field("Analyser", analyser);
    }

    if let Some(text) = additional_text {
        embed.with_description(text);
    }

    if is_debug {
        embed.with_color(255, 0, 0);
    }

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

pub fn create_deleted_downvoted_comment(comment: &CreatedCommentWithLinkInfo) -> Message {
    Message::builder().embed(
        MessageEmbed::builder()
            .title("Removed downvoted post")
            .description(format!("For {}", comment.link_title()))
            .reddit_link(comment.permalink()),
    )
}
