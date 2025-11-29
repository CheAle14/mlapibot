use std::collections::HashSet;

use roux::{
    api::{ThingFullname, subreddit::ModActionType},
    client::RedditClient,
    models::SubmissionLinkInfo,
};

use crate::{
    QuickStopError,
    client::{
        ModuleRedditClient,
        module::{InboxAction, InboxMsg, Module},
    },
    exts::DetectionExt,
    subreddit::Subreddit,
    webhook::create_error_processing_message,
};

pub struct InboxCommands;

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

    fn run_inbox<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddits: &mut [Subreddit],
        item: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<super::InboxAction>> {
        if item.subject == "test" {
            client.run_inbox_test(&item)?;
        } else if item.subject == "redo" {
            return client.try_redo_from_message(subreddits, &item);
        } else if item.subject == "media" {
            client.try_run_media_count(subreddits, &item)?;
        } else if item.subject == "removal_reasons" {
            client.send_removal_reasons(&item)?;
        } else if item.subject == "sticky" {
            client.try_sticky_status_post(subreddits, &item)?;
        } else if item.subject.trim().eq_ignore_ascii_case("stop") {
            client.try_stop_bot(subreddits, &item)?;
        } else if item.author == "" {
            if let Some(subreddit) = item.subject.strip_prefix("invitation to moderate /r/") {
                let sub = client.client.subreddit(subreddit);
                if let Err(e) = sub.accept_moderator_invite() {
                    println!("Unable to accept mod: {e:?}");
                }
            }
        }

        Ok(None)
    }
}

impl<'client> ModuleRedditClient<'client> {
    fn run_inbox_test(&mut self, message: &InboxMsg<'_>) -> anyhow::Result<()> {
        let mut warnings = Vec::new();
        let ctx = mlapibot_analysis::Context::new_body(message.body, &mut warnings)?;

        self.send_warnings(warnings, "Warnings in inbox test")?;

        match mlapibot_analysis::get_best_analysis(&ctx, &self.analzyers) {
            Ok(Some((detection, detected))) => {
                let text = detection.get_markdown(&ctx)?;
                let text = text.join("\n\n\n> ");
                let s = format!("Detected {:?}. Full text:\r\n\r\n> {text}", detected.name);
                message.reply(&s)?;
            }
            Ok(None) => {
                let mut text = String::from("No scams were detected, text was:\r\n\r\n");
                for img in &ctx.images {
                    text.push_str("> ");
                    text.push_str(&img.full_text());
                    text.push_str("\n\n\n");
                }
                message.reply(&text)?;
            }
            Err(err) => {
                eprintln!(
                    "Error whilst analyising message {:?}: {err:?}",
                    message.subject
                );
                if let Some(webhook) = &mut self.webhook {
                    let msg = create_error_processing_message(message.author, message.subject);
                    webhook.send(&msg)?;
                }
                message.reply(
                    "An internal error occured whilst attempting to process your request. Sorry!",
                )?;
            }
        };
        Ok(())
    }

    fn try_stop_bot(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<()> {
        for sub in subreddits {
            if sub.is_moderator(message.author)? {
                let _ = message.reply("Stopping...");

                return Err(anyhow::Error::from(QuickStopError));
            }
        }

        message.reply("You are authorised to do that")?;

        Ok(())
    }

    fn try_run_media_count(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<()> {
        use std::fmt::Write;

        let sub = self.client.subreddit(message.body);

        let Some(our_sub) = subreddits.iter_mut().find(|s| s.name() == &sub.name) else {
            message.reply("That subreddit is not managed by this bot.")?;
            return Ok(());
        };

        if !our_sub.is_moderator(message.author)? {
            message.reply("You are not a moderator of that subreddit!")?;
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
            let page = sub.list_mod_log(after.clone(), Some(500), None, None)?;

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

        message.reply(&output)?;

        Ok(())
    }

    fn try_redo_from_message(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<InboxAction>> {
        let Ok(link) = SubmissionLinkInfo::parse(message.body) else {
            message.reply("Unrecognised link. Must be a full link to a submission or comment.")?;
            return Ok(None);
        };

        let (submission, comment) = match link.comment_id {
            None => {
                let submission = match self.client.get_submission_by_info(&link) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "Failed to parse or fetch post to redo: {:?} {e:?}",
                            message.body
                        );
                        message.reply("Failed to parse or fetch which submission you meant.")?;
                        return Ok(None);
                    }
                };

                (submission, None)
            }
            Some(comment_id) => {
                match self.client.article_and_comments(
                    link.subreddit,
                    link.post_id,
                    comment_id,
                    None,
                    None,
                ) {
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
                        message.reply("Failed to parse or fetch which comment you meant.")?;
                        return Ok(None);
                    }
                }
            }
        };

        let Some(subreddit) = subreddits
            .iter_mut()
            .find(|s| s.name() == submission.subreddit())
        else {
            message.reply("That subreddit is not managed by this bot.")?;
            return Ok(None);
        };

        if !subreddit.is_moderator(message.author)? {
            message.reply("You are not a moderator of that subreddit!")?;
            return Ok(None);
        }

        match comment {
            Some(comment) => Ok(Some(InboxAction::RedoMsg(submission, comment))),
            None => Ok(Some(InboxAction::RedoSub(submission))),
        }
    }

    fn try_sticky_status_post(
        &mut self,
        subreddits: &mut [Subreddit],
        message: &InboxMsg<'_>,
    ) -> anyhow::Result<()> {
        let submission = match self.client.get_submission_by_link(message.body) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse or fetch post to redo: {:?} {e:?}",
                    message.body
                );
                message.reply("Failed to parse or fetch which submission you meant.")?;
                return Ok(());
            }
        };

        if submission.stickied() {
            message.reply("Submission is already stickied. You can simply un-sticky it if you want to remove it.")?;
            return Ok(());
        }

        let Some(subreddit) = subreddits
            .iter_mut()
            .find(|s| s.name() == submission.subreddit())
        else {
            message.reply("That subreddit is not managed by this bot.")?;
            return Ok(());
        };

        if !subreddit.is_moderator(message.author)? {
            message.reply("You are not a moderator of that subreddit!")?;
            return Ok(());
        }

        let Some(config) = self.subreddits_config.get_status(subreddit.name()) else {
            message.reply("That subreddit is not configured for automatic status posts.")?;
            return Ok(());
        };

        let Some(sticky) = config.sticky.as_ref() else {
            message.reply("That subreddit is not configured for stickying its status posts.")?;
            return Ok(());
        };

        let Some(_exists) = self.db.get_incident_from_post(submission.name().full())? else {
            message.reply("That submission was not submitted by this bot for status tracking.")?;
            return Ok(());
        };

        subreddit.sticky_incident_post(self.db, sticky, &submission)?;

        message.reply("✔ That post should now be stickied. It will be automatically un-stickied some time after the incident is resolved.")?;

        Ok(())
    }

    fn send_removal_reasons(&mut self, message: &InboxMsg<'_>) -> anyhow::Result<()> {
        use std::fmt::Write;

        let subreddit = message.body.trim().trim_start_matches("/r/");
        let mut sending = format!("Removal reasons for /r/{subreddit}:  \n\n");
        let subreddit = self.client.subreddit(subreddit);

        let reasons = subreddit.list_removal_reasons()?;

        for id in reasons.order {
            let _ = match reasons.data.get(&id) {
                Some(reason) => writeln!(sending, "- {}: {}  ", reason.id, reason.title),
                None => writeln!(sending, "- {id}: <not found>  "),
            };
        }

        message.reply(&sending)?;

        Ok(())
    }
}
