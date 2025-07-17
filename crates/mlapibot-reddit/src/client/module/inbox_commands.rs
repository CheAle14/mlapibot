use std::collections::HashSet;

use mlapibot_common::LowercaseString;
use roux::{
    api::{ThingFullname, subreddit::ModActionType},
    client::RedditClient,
};

use crate::{
    RedditMessage,
    client::{
        ModuleRedditClient,
        module::{InboxAction, Module},
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
        item: &RedditMessage,
        author: &str,
        subject: &str,
    ) -> anyhow::Result<Option<super::InboxAction>> {
        if subject == "test" {
            client.run_inbox_test(&item)?;
        } else if subject == "redo" {
            return client.try_redo_from_message(subreddits, author, &item);
        } else if subject == "media" {
            client.try_run_media_count(subreddits, author, &item)?;
        } else if subject == "removal_reasons" {
            client.send_removal_reasons(&item)?;
        } else if author == "" {
            if let Some(subreddit) = subject.strip_prefix("invitation to moderate /r/") {
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
    fn run_inbox_test(&mut self, message: &RedditMessage) -> anyhow::Result<()> {
        let mut warnings = Vec::new();
        let ctx = mlapibot_analysis::Context::new_body(message.body(), &mut warnings)?;

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
                    message.subject()
                );
                if let Some(webhook) = &mut self.webhook {
                    let msg = create_error_processing_message(&message);
                    webhook.send(&msg)?;
                }
                message.reply(
                    "An internal error occured whilst attempting to process your request. Sorry!",
                )?;
            }
        };
        Ok(())
    }

    fn try_run_media_count(
        &mut self,
        subreddits: &mut [Subreddit],
        author: &str,
        message: &RedditMessage,
    ) -> anyhow::Result<()> {
        use std::fmt::Write;

        let sub = self.client.subreddit(message.body());

        let our_sub = subreddits.iter_mut().find(|s| s.name() == &sub.name);
        match our_sub {
            Some(sub) => {
                if !sub.is_moderator(author)? {
                    message.reply("You are not a moderator of that subreddit!")?;
                    return Ok(());
                }
            }
            None => {
                message.reply("I am not monitoring that subreddit!")?;
                return Ok(());
            }
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
        author: &str,
        message: &RedditMessage,
    ) -> anyhow::Result<Option<InboxAction>> {
        let submission = match self.client.get_submission_by_link(message.body()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse or fetch post to redo: {:?} {e:?}",
                    message.body()
                );
                message.reply("Failed to parse or fetch which submission you meant.")?;
                return Ok(None);
            }
        };

        let mut is_mod = false;
        let mods = self
            .client
            .subreddit(submission.subreddit().as_str())
            .moderators()?;
        for moderator in mods.data.children {
            if author == moderator.name {
                is_mod = true;
                break;
            }
        }

        if !is_mod {
            eprintln!(
                "  {author} attempted unauthorized redo of {}",
                submission.permalink()
            );
            return Ok(None);
        }

        let name = LowercaseString::new(submission.subreddit());
        if !subreddits.iter().any(|s| s.name() == &name) {
            message.reply("That subreddit is not monitored")?;
            return Ok(None);
        };

        return Ok(Some(InboxAction::Redo(submission)));
    }

    fn send_removal_reasons(&mut self, message: &RedditMessage) -> anyhow::Result<()> {
        use std::fmt::Write;

        let subreddit = message.body().trim().trim_start_matches("/r/");
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
