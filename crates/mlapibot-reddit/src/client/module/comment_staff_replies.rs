use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use mlapibot_datastore::staff_replies::StaffReply;
use mlapibot_markdown::substr::{LayoutPlan, SubstrAttempt, substr_markdown_many};
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

fn construct_layout_plan(
    all_replies: &[StaffReply],
    subreddit: &str,
    post_id: &str,
) -> Result<LayoutPlan, std::fmt::Error> {
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

    // Fortunately, we can use a shortened version for {link}:
    // /r/{SUBREDDIT}/comments/{POST_ID}/-/{COMMENT_ID}/

    let mut plan = LayoutPlan::new();

    if all_replies.len() == 1 {
        writeln!(plan, "There is 1 staff reply in this thread:")?;
    } else {
        writeln!(
            plan,
            "There are {} staff replies in this thread:",
            all_replies.len()
        )?;
    }

    for reply in all_replies {
        plan.argument(|arg| {
            writeln!(
                arg,
                "\n[By {username}](/r/{subreddit}/comments/{post_id}/-/{comment_id}?context=9):\n",
                username = reply.author_name,
                comment_id = reply.comment_id
            )?;

            write!(arg, "> ")?;
            arg.placeholder();

            writeln!(arg, "\n\n---")
        })?;
    }

    Ok(plan)
}

#[derive(Debug, thiserror::Error)]
enum LayoutReplyError {
    #[error("template size {0} is beyond max_len")]
    TemplateTooLarge(usize),
}

/// Returns an error if it is not possible to fit the items in `max_len`.
fn layout_reply(
    all_replies: &[StaffReply],
    subreddit: &str,
    post_id: &str,
    max_len: usize,
) -> Result<String, LayoutReplyError> {
    let plan = construct_layout_plan(all_replies, subreddit, post_id)
        .expect("write into string should suceed");

    if plan.len() > max_len {
        return Err(LayoutReplyError::TemplateTooLarge(plan.len()));
    }

    let diff = max_len - plan.len();

    let reply_texts = substr_markdown_many(all_replies.iter().map(|r| r.content.as_str()), diff);

    struct Item<'a> {
        reply: &'a StaffReply,
        substr: &'a SubstrAttempt<'a>,
    }

    impl<'a> std::fmt::Display for Item<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut first = true;
            for line in self.substr.text().lines() {
                if first {
                    first = false;
                    f.write_str(line)?;
                } else {
                    f.write_str("\n")?;
                    // Technically, this means the layout plan is wrong by two characters for every line
                    // However, we provide a buffer of ~500 characters between the actual max comment length
                    // so this should be fine.
                    write!(f, "> {line}")?;
                }
            }

            if self.reply.content.len() != self.substr.text().len() {
                f.write_str(" [..]")?;
            }

            Ok(())
        }
    }

    let items = all_replies
        .iter()
        .zip(&reply_texts)
        .map(|(reply, substr)| Item { reply, substr });

    Ok(plan.execute(items))
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

        let reply_text = layout_reply(&all_replies, subreddit, post_id, 9500)
            .with_context(|| format!("staff reply /r/{subreddit}/{post_id}"))?;

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

        if let Some(live) = client.db.get_staff_reply_thread(comment.link_id().id())? {
            // Normally we ignore our own comments, so the only way this could've triggered
            // is if someone used the `redo` command and gave our comment as the link.
            println!(
                "Refreshing staff reply {}/{}",
                comment.subreddit_name_prefixed(),
                live.post_id
            );
            self.update_or_make_staff_reply_comment(client, comment.subreddit(), &live.post_id)
                .context("redo reply")?;
            return Ok(());
        }

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

    static SUBREDDIT: &str = "subreddit1";
    static POST_ID: &str = "post123";

    fn test_replies() -> Vec<StaffReply> {
        vec![
            StaffReply {
                comment_id: String::from("comment0"),
                post_id: POST_ID.to_owned(),
                author_name: String::from("user0"),
                content: "1234567890".repeat(10),
                ..Default::default()
            },
            StaffReply {
                comment_id: String::from("comment1"),
                post_id: POST_ID.to_owned(),
                author_name: String::from("user1"),
                content: String::from("_Old_  \n**New**"),
                ..Default::default()
            },
        ]
    }

    #[test]
    pub fn constructs_staff_reply_layout_plan() {
        static EXPECTED: &str = include_str!("expected_plan.test.txt");

        let comments = test_replies();
        let plan = super::construct_layout_plan(&comments, SUBREDDIT, POST_ID).unwrap();
        assert_eq!(plan.template_string(), EXPECTED);
    }

    #[test]
    pub fn lays_out_staff_reply_comment() {
        static EXPECTED: &str = include_str!("expected_reply.test.txt");

        let comments = test_replies();
        let output = super::layout_reply(&comments, SUBREDDIT, POST_ID, 222).unwrap();
        assert_eq!(output, EXPECTED);
    }
}
