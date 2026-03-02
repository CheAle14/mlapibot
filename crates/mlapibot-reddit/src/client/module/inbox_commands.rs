use std::collections::HashSet;

use mlapibot_database_v2::repos::{incidents::IncidentRepo, staff_replies::StaffReplyRepo};
use roux::{
    api::{ThingFullname, subreddit::ModActionType},
    client::{AuthedClient, RedditClient},
    models::{LatestComment, SubmissionLinkInfo},
};

use crate::{
    QuickStopError, Submission,
    client::{
        ModuleRedditClient,
        module::{InboxAction, InboxMsg, Module},
    },
    exts::DetectionExt,
    subreddit::Subreddit,
    webhook::create_error_processing_message,
};

pub struct InboxCommands;

#[async_trait::async_trait(?Send)]
impl Module for InboxCommands {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "inbox_commands"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::INBOX
    }

    async fn run_inbox<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddits: &mut [Subreddit],
        item: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<super::InboxAction>> {
        if item.subject == "test" {
            client.run_inbox_test(&item).await?;
        } else if item.subject == "redo" {
            return client.try_redo_from_message(subreddits, &item).await;
        } else if item.subject == "media" {
            client.try_run_media_count(subreddits, &item).await?;
        } else if item.subject == "removal_reasons" {
            client.send_removal_reasons(&item).await?;
        } else if item.subject == "sticky" {
            client.try_sticky_status_post(subreddits, &item).await?;
        } else if item.subject == "suffix" {
            return client.run_staff_reply_suffix(subreddits, &item).await;
        } else if item.author == "" {
            if let Some(subreddit) = item.subject.strip_prefix("invitation to moderate /r/") {
                let sub = client.client.subreddit(subreddit);
                if let Err(e) = sub.accept_moderator_invite().await {
                    println!("Unable to accept mod: {e:?}");
                }
            }
        }

        Ok(None)
    }
}

struct LinkData<'a> {
    submission: Submission,
    comment: Option<LatestComment<AuthedClient>>,
    subreddit: &'a mut Subreddit,
}

impl<'client> ModuleRedditClient<'client> {
    async fn parse_link<'msg, 'sub>(
        &mut self,
        subreddits: &'sub mut [Subreddit],
        message: &InboxMsg<'msg>,
    ) -> anyhow::Result<Option<(LinkData<'sub>, &'msg str)>> {
        let (first_line, rest) = match message.body.split_once('\n') {
            Some((fl, rest)) => (fl.trim_end(), rest),
            None => (message.body, ""),
        };

        let Ok(link) = SubmissionLinkInfo::parse(first_line) else {
            message
                .reply("Unrecognised link. Must be a full link to a submission or comment.")
                .await?;
            return Ok(None);
        };

        let (submission, comment) = match link.comment_id {
            None => {
                let submission = match self.client.get_submission_by_info(&link).await {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "Failed to parse or fetch post to redo: {:?} {e:?}",
                            message.body
                        );
                        message
                            .reply("Failed to parse or fetch which submission you meant.")
                            .await?;
                        return Ok(None);
                    }
                };

                (submission, None)
            }
            Some(comment_id) => {
                match self
                    .client
                    .article_and_comments(link.subreddit, link.post_id, comment_id, None, None)
                    .await
                {
                    Ok((sub, comments)) => {
                        let comment = comments.into_iter().next().expect("direct link to comment");
                        let comment = comment.into_latest(&sub);
                        (sub, Some(comment))
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to parse or fetch comment to redo: {:?} {e:?}",
                            message.body
                        );
                        message
                            .reply("Failed to parse or fetch which comment you meant.")
                            .await?;
                        return Ok(None);
                    }
                }
            }
        };

        let Some(subreddit) = subreddits
            .iter_mut()
            .find(|s| s.name() == submission.subreddit())
        else {
            message
                .reply("That subreddit is not managed by this bot.")
                .await?;
            return Ok(None);
        };

        if !subreddit.is_moderator(message.author) {
            message
                .reply("You are not a moderator of that subreddit!")
                .await?;
            return Ok(None);
        }

        Ok(Some((
            LinkData {
                submission,
                comment,
                subreddit,
            },
            rest,
        )))
    }

    async fn run_staff_reply_suffix(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<InboxAction>> {
        let Some((link, suffix)) = self.parse_link(subreddits, message).await? else {
            return Ok(None);
        };

        let Some(comment) = link.comment else {
            message
                .reply("You must provide a link to the bot's comment.")
                .await?;
            return Ok(None);
        };

        let suffix = if suffix.trim().is_empty() {
            None
        } else {
            Some(suffix.trim())
        };

        if let Err(err) = self
            .db
            .update_staff_reply_thread_suffix(&link.submission.id(), suffix)
            .await
        {
            eprintln!("failed to set staff reply thread prefix: {err}");
            message
                .reply("Failed to set suffix, probably not a staff reply thread comment")
                .await?;
            return Ok(None);
        }

        message
            .reply("✔ Suffix set. The message should be edited soon.")
            .await?;

        // Trigger the staff reply module to edit the message.
        Ok(Some(InboxAction::RedoMsg(link.submission, comment)))
    }

    async fn run_inbox_test(&mut self, message: &InboxMsg<'_>) -> anyhow::Result<()> {
        let mut warnings = Vec::new();
        let ctx = mlapibot_analysis::Context::new_body(message.body, &mut warnings).await?;

        self.send_warnings(warnings, "Warnings in inbox test")
            .await?;

        match mlapibot_analysis::get_best_analysis(&ctx, &self.analzyers) {
            Ok(Some((detection, detected))) => {
                let text = detection.get_markdown(&ctx)?;
                let text = text.join("\n\n\n> ");
                let s = format!("Detected {:?}. Full text:\r\n\r\n> {text}", detected.name);
                message.reply(&s).await?;
            }
            Ok(None) => {
                let mut text = String::from("No scams were detected, text was:\r\n\r\n");
                for img in &ctx.images {
                    text.push_str("> ");
                    text.push_str(&img.full_text());
                    text.push_str("\n\n\n");
                }
                message.reply(&text).await?;
            }
            Err(err) => {
                eprintln!(
                    "Error whilst analyising message {:?}: {err:?}",
                    message.subject
                );
                if let Some(webhook) = &mut self.webhook {
                    let msg = create_error_processing_message(message.author, message.subject);
                    webhook.send(&msg).await?;
                }
                message.reply(
                    "An internal error occured whilst attempting to process your request. Sorry!",
                ).await?;
            }
        };
        Ok(())
    }

    async fn try_run_media_count(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<()> {
        use std::fmt::Write;

        let sub = self.client.subreddit(message.body);

        let Some(our_sub) = subreddits.iter_mut().find(|s| s.name() == sub.name()) else {
            message
                .reply("That subreddit is not managed by this bot.")
                .await?;
            return Ok(());
        };

        if !our_sub.is_moderator(message.author) {
            message
                .reply("You are not a moderator of that subreddit!")
                .await?;
            return Ok(());
        }

        let mut removed_ids: HashSet<ThingFullname> = HashSet::new();
        let mut approved_ids: HashSet<ThingFullname> = HashSet::new();

        let mut removed_media = 0;
        let mut approved_media = 0;
        let mut unknown_media = 0;

        let utc_end = 1740096000.0;
        let mut after = None;

        let mut output = String::with_capacity(128);

        'outer: loop {
            let page = sub
                .list_mod_log(after.clone(), Some(500), None, None)
                .await?;

            for (idx, action) in page.into_iter().enumerate() {
                let Some(fullname) = action.target_fullname else {
                    continue;
                };

                if action.moderator != "AutoModerator" {
                    match action.action {
                        ModActionType::RemoveComment => {
                            removed_ids.insert(fullname);
                        }
                        ModActionType::ApproveComment => {
                            approved_ids.insert(fullname);
                        }
                        _ => (),
                    };
                } else if action.details == "Media in comments" {
                    if removed_ids.contains(&fullname) {
                        removed_media += 1;
                    } else if approved_ids.contains(&fullname) {
                        approved_media += 1;
                    } else {
                        unknown_media += 1;
                        let _ = writeln!(output, "unknown: {:?}  ", action.target_permalink);
                    }
                }

                if idx == 499 {
                    after = Some(action.id);
                }

                if action.created_utc < utc_end {
                    break 'outer;
                }
            }
        }

        let total = removed_media + approved_media + unknown_media;
        let _ = writeln!(
            output,
            "\nFound {total} media in comments.\nRemoved: {removed_media}\nApproved: {approved_media}\nUnknown: {unknown_media}"
        );

        message.reply(&output).await?;

        Ok(())
    }

    async fn try_redo_from_message(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<InboxAction>> {
        let Some((link, _)) = self.parse_link(subreddits, message).await? else {
            return Ok(None);
        };

        match link.comment {
            Some(comment) => Ok(Some(InboxAction::RedoMsg(link.submission, comment))),
            None => Ok(Some(InboxAction::RedoSub(link.submission))),
        }
    }

    async fn try_sticky_status_post(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<()> {
        let Some((link, _)) = self.parse_link(subreddits, message).await? else {
            return Ok(());
        };

        if link.submission.stickied() {
            message.reply("Submission is already stickied. You can simply un-sticky it if you want to remove it.").await?;
            return Ok(());
        }

        if !link.subreddit.db.mod_status.enabled {
            message
                .reply("That subreddit is not configured for automatic status posts.")
                .await?;
            return Ok(());
        }

        let Some(sticky) = link.subreddit.db.mod_status.sticky.as_ref() else {
            message
                .reply("That subreddit is not configured for stickying its status posts.")
                .await?;
            return Ok(());
        };

        let Some(_exists) = self
            .db
            .get_incident_post_by_id(link.submission.name().full())
            .await?
        else {
            message
                .reply("That submission was not submitted by this bot for status tracking.")
                .await?;
            return Ok(());
        };

        Subreddit::sticky_incident_post(
            &mut link.subreddit.reddit,
            self.db,
            sticky,
            &link.submission,
        )
        .await?;

        message.reply("✔ That post should now be stickied. It will be automatically un-stickied some time after the incident is resolved.").await?;

        Ok(())
    }

    async fn send_removal_reasons(&mut self, message: &InboxMsg<'_>) -> anyhow::Result<()> {
        use std::fmt::Write;

        let subreddit = message.body.trim().trim_start_matches("/r/");
        let mut sending = format!("Removal reasons for /r/{subreddit}:  \n\n");
        let subreddit = self.client.subreddit(subreddit);

        let reasons = subreddit.list_removal_reasons().await?;

        for id in reasons.order {
            let _ = match reasons.data.get(&id) {
                Some(reason) => writeln!(sending, "- {}: {}  ", reason.id, reason.title),
                None => writeln!(sending, "- {id}: <not found>  "),
            };
        }

        message.reply(&sending).await?;

        Ok(())
    }
}
