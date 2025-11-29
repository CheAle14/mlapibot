use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use mlapibot_datastore::staff_replies::StaffReply;
use mlapibot_markdown::substr::substr_markdown_many;
use roux::{
    api::{ArticleCommentData, ArticleCommentOrMoreComments, ArticleReplies, ThingFullname},
    client::{AuthedClient, RedditClient},
    models::{ArticleCommentOrMore, Listing},
};

use crate::client::{ModuleRedditClient, module::impl_mask_subreddits};

pub struct CommentStaffReplies;

fn num_digits(n: usize) -> usize {
    n.checked_ilog10().unwrap_or(0) as usize + 1
}

fn layout_reply(
    all_replies: &[StaffReply],
    subreddit: &str,
    post_id: &str,
    max_len: usize,
) -> String {
    use std::fmt::Write;

    // Output is roughly:
    //
    //     There are {N} staff replies in this thread:
    //
    //     [By {username}]({link}):
    //
    //     > {content}
    //
    //     ---
    //
    //     [By {username2}]({link}):
    //
    //     > {content1}
    //     > {content2}

    // Fortunately, the {link} is relatively constant:
    // /r/{SUBREDDIT}/comments/{POST_ID}/-/{COMMENT_ID}/

    let link_base_len =
        "/r/".len() + subreddit.len() + "/comments/".len() + post_id.len() + "/-/".len();

    let len: usize = 41 // "There are..." to "...thread:"
        + num_digits(all_replies.len()) // {N}
        + all_replies.iter().map(|v|
            10 // "By" text, syntax characters, some newlines
            +
            v.author_name.len()
            +
            (link_base_len + v.comment_id.len())
            +
            (1 * v.num_newlines()) // adding ">" to each line to ensure quoted
        ).sum::<usize>();

    let reply_texts = substr_markdown_many(
        all_replies.iter().map(|r| r.content.as_str()),
        max_len.checked_sub(len).unwrap_or(1000),
    );

    let mut output_text = format!(
        "There are {N} staff replies in this thread:\n",
        N = all_replies.len()
    );

    for (reply, substr) in all_replies.iter().zip(reply_texts) {
        let _ = writeln!(
            output_text,
            "\n[By {username}](/r/{subreddit}/comments/{post_id}/-/{comment_id}):",
            username = reply.author_name,
            comment_id = reply.comment_id,
        );

        for line in substr.text().lines() {
            output_text.push_str("\n>");
            output_text.push_str(line);
        }

        if substr.text().len() != reply.content.len() {
            // we cut it down
            output_text.push_str(" [...]");
        }

        output_text.push_str("\n\n---")
    }

    output_text
}

pub fn visit_comments<V, E>(
    listing: &Listing<ArticleCommentOrMore<AuthedClient>>,
    mut visitor: V,
) -> Result<(), E>
where
    V: FnMut(&ArticleCommentData) -> Result<(), E>,
{
    let mut queue = VecDeque::new();

    for comment in &listing.children {
        match comment {
            ArticleCommentOrMore::Comment(comment) => {
                visitor(comment.raw_data())?;

                if let ArticleReplies::Replies(replies) = comment.replies() {
                    for comment in &replies.data.children {
                        queue.push_back(comment);
                    }
                }
            }
            ArticleCommentOrMore::More(..) => {}
        }
    }

    while let Some(maybe_reply) = queue.pop_back() {
        match maybe_reply {
            ArticleCommentOrMoreComments::Comment(comment) => {
                visitor(comment)?;

                if let ArticleReplies::Replies(replies) = &comment.replies {
                    for comment in &replies.data.children {
                        queue.push_back(comment);
                    }
                }
            }
            ArticleCommentOrMoreComments::More(..) => {}
        }
    }

    Ok(())
}

impl CommentStaffReplies {
    fn fetch_reply_updates(
        &self,
        client: &mut ModuleRedditClient,
        subreddit: &str,
        post_id: &str,
        replies: &mut [StaffReply],
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        // Best case scenario is that the post doesn't have too many comments, so
        // we can just fetch the entire post's comments and then look for the
        // staff replies we know about and update our record.
        //
        // Any left over we can fetch one-by-one.

        let mut reply_map = HashMap::new();
        for reply in replies {
            reply_map.insert(reply.comment_id.clone(), reply);
        }

        let post_comments = client.client.article_comments(
            subreddit,
            &ThingFullname::from_submission_id(post_id),
            None,
            Some(1000),
        )?;

        visit_comments::<_, anyhow::Error>(&post_comments, |comment| {
            if let Some(staff_reply) = reply_map.remove(&comment.common.id) {
                staff_reply.content = comment.common.body.to_owned();
                staff_reply.last_updated = now;

                if staff_reply.author_name != comment.common.author {
                    staff_reply.author_name = comment.common.author.clone();
                }

                client
                    .db
                    .update_staff_reply(&staff_reply)
                    .context("update staff reply")?;
            }

            Ok(())
        })?;

        for (_, unseen) in reply_map {
            // we don't care about unseen ones that we don't think are outdated.
            if unseen.is_outdated(now) {
                println!("TODO: fetch individual comment {unseen:?}");
            }
        }

        Ok(())
    }

    fn update_or_make_staff_reply_comment(
        &self,
        client: &mut ModuleRedditClient,
        subreddit: &str,
        post_id: &str,
    ) -> anyhow::Result<()> {
        let mut all_replies = client.db.get_staff_replies_in(post_id)?;
        let now = Utc::now();

        if all_replies.iter().any(|v| v.is_outdated(now)) {
            self.fetch_reply_updates(client, subreddit, post_id, &mut all_replies, now)
                .with_context(|| format!("fetch replies for /r/{subreddit}/{post_id}"))?;
        }

        let reply_text = layout_reply(&all_replies, subreddit, post_id, 9500);

        match client.db.get_staff_reply_thread(post_id)? {
            Some(existing) => {
                let fullname = ThingFullname::from_comment_id(&existing.our_comment_id);
                client.client.edit(&reply_text, &fullname)?;
            }
            None => {
                let fullname = ThingFullname::from_submission_id(post_id);
                let reply = client.client.comment(&reply_text, &fullname)?;

                client.db.insert_staff_reply_thread(post_id, reply.id())?;

                if reply.can_mod_post() {
                    reply.distinguish(roux::models::Distinguish::Moderator, true)?;
                    reply.lock()?;
                }
            }
        }

        Ok(())
    }
}

impl super::Module for CommentStaffReplies {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "comment_staff_replies"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::COMMENTS
    }

    impl_mask_subreddits!(comments_staff_reply => comments);

    fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        let Some(config) = client.subreddits_config.get(&comment.subreddit().into()) else {
            return Ok(());
        };

        let Some(config) = config.comments_staff_reply.as_ref() else {
            return Ok(());
        };

        for title_filter in &config.ignore_post_title_contains {
            if comment.link_title().contains(title_filter) {
                println!("comment in a thread with rejected title: {title_filter:?}");
                return Ok(());
            }
        }

        if !config.is_staff(
            comment.author_flair_template_id(),
            comment.author_flair_css_class(),
        ) {
            println!(
                "comment author not staff flaired: template_id = {:?}; css_class = {:?}",
                comment.author_flair_template_id(),
                comment.author_flair_css_class()
            );

            return Ok(());
        }

        let comment_id = comment.id();
        let post_id = comment.link_id().id();

        client
            .db
            .insert_staff_reply(comment_id, post_id, comment.author(), comment.body())
            .with_context(|| format!("staff reply {post_id} / {comment_id}"))?;

        self.update_or_make_staff_reply_comment(client, comment.subreddit(), post_id)
            .with_context(|| format!("make reply comment {post_id} (due to {comment_id})"))?;

        Ok(())
    }

    fn run_timer<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddits: &mut [crate::subreddit::Subreddit],
    ) -> anyhow::Result<std::time::Duration> {
        println!("Timer!");
        Ok(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use mlapibot_datastore::staff_replies::StaffReply;

    #[test]
    pub fn lays_out_staff_reply_comment() {
        static EXPECTED: &str = include_str!("expected_reply.txt");

        let post_id = String::from("post123");
        let comments = vec![StaffReply {
            comment_id: String::from("comment0"),
            post_id: post_id.clone(),
            author_name: String::from("user0"),
            content: "1234567890".repeat(10),
            ..Default::default()
        }];

        let output = super::layout_reply(&comments, "subreddit1", &post_id, 114);
        assert_eq!(output, EXPECTED);
    }
}
