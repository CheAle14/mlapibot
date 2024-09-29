use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError},
};

use anyhow::{bail, Context};
use config::{SubredditModerateConfig, SubredditsConfig};
use flairs::{FlairChangeConfig, SubredditFlairConfig};
use roux::{
    api::ThingId,
    client::{OAuthClient, RedditClient as RouxRedditClient},
    models::Distinguish,
};
use status_tracker::CachedIncidentSubmissions;
use statuspage::StatusClient;
use subreddit::Subreddit;
use tera::Tera;

use crate::{
    analysis::{self, get_best_analysis, Analyzer},
    context,
    imgur::{self, ImgurClient},
    tryw,
    utils::{is_debug, LowercaseString, SubmissionExt},
    webhook::{
        create_deleted_downvoted_comment, create_detection_message,
        create_error_processing_message, create_error_processing_post,
        create_generic_error_message, create_inbox_message, create_multiple_error_message,
        Message as DiscordMessage, WebhookClient,
    },
    RedditInfo,
};

pub mod config;
mod flairs;
mod ratelimiter;
mod seen_tracker;
mod status_tracker;
mod subreddit;

pub type RouxClient = roux::client::AuthedClient;
pub type Submission = roux::models::Submission<RouxClient>;
pub type Comment = roux::models::ArticleComment<RouxClient>;
pub type RedditMessage = roux::models::Message<RouxClient>;
pub type CreatedComment = roux::models::CreatedComment<RouxClient>;
pub type CreatedCommentWithLinkInfo = roux::models::CreatedCommentWithLinkInfo<RouxClient>;

pub struct RedditClient<'a> {
    data_dir: PathBuf,
    analzyers: &'a [Analyzer],
    client: RouxClient,
    subreddits: Vec<Subreddit>,
    ratelimit: ratelimiter::Ratelimiter,
    templates: Tera,
    webhook: Option<WebhookClient>,
    imgur: Option<ImgurClient>,
    status: StatusClient,
    subreddits_config: SubredditsConfig,
    dry_run: bool,
    status_webhook: Option<String>,
    admin: Option<String>,
}

impl<'a> RedditClient<'a> {
    const USER_AGENT: &'static str = "rust-mlapibot-ocr by /u/DarkOverLordCO";

    pub fn new(analzyers: &'a [Analyzer], args: &RedditInfo) -> anyhow::Result<Self> {
        let templates_path = args.data_dir.join("templates").join("*.md");
        let templates = Tera::new(templates_path.as_os_str().to_str().unwrap())?;
        let found: Vec<_> = templates.get_template_names().collect();
        assert!(found.len() > 0);
        let credentials = args.get_credentials()?;

        let config = roux::Config::new(
            Self::USER_AGENT,
            &credentials.client_id,
            &credentials.client_secret,
        )
        .username(&credentials.username)
        .password(&credentials.password);

        let client = OAuthClient::new(config)?.login()?;

        let webhook = credentials
            .webhook_url
            .as_ref()
            .map(|url| WebhookClient::new(url))
            .transpose()?;

        let imgur = credentials
            .imgur_credentials
            .as_ref()
            .map(|creds| ImgurClient::new(creds))
            .transpose()?;

        let status = StatusClient::new("https://discordstatus.com")?;

        let subreddits_config = args.get_subreddits_config()?;

        let mut subreddit_names = HashSet::new();
        for sub in &args.subreddits {
            subreddit_names.insert(sub.clone());
        }
        for key in subreddits_config.keys() {
            subreddit_names.insert(key.clone());
        }

        let subreddits: Vec<Subreddit> = subreddit_names
            .into_iter()
            .map(|name| Subreddit::new(args, client.subreddit(name.as_str()), name))
            .collect();

        println!(
            "Logged in as /u/{}; monitoring {} with {} total known subreddits in {}",
            credentials.username,
            args.subreddits.len(),
            subreddits_config.len(),
            if is_debug() {
                "debug mode"
            } else {
                "release mode"
            }
        );

        if args.dry_run {
            println!("Running in dry-run mode.");
        }

        Ok(Self {
            client,
            subreddits,
            analzyers,
            ratelimit: ratelimiter::Ratelimiter::new(),
            data_dir: args.scratch_dir.clone(),
            templates,
            webhook,
            imgur,
            status,
            subreddits_config,
            dry_run: args.dry_run,
            status_webhook: args.status_webhook.clone(),
            admin: args.admin.clone(),
        })
    }

    fn _send_warnings(
        webhook: &mut Option<WebhookClient>,
        warnings: Vec<anyhow::Error>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        if warnings.len() > 0 {
            if let Some(webhook) = webhook {
                let message = create_multiple_error_message(context, warnings);
                webhook.send(&message)?;
            }
        }

        Ok(())
    }

    fn send_warnings(
        &mut self,
        warnings: Vec<anyhow::Error>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        Self::_send_warnings(&mut self.webhook, warnings, context)
    }

    fn run_inbox_test(&mut self, message: &RedditMessage) -> anyhow::Result<()> {
        let (ctx, warnings) = tryw!(
            crate::context::Context::from_direct_message(message),
            Result::Err
        );

        self.send_warnings(warnings, "Warnings in inbox test")?;

        match get_best_analysis(&ctx, &self.analzyers) {
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

    fn redo_from_message(&mut self, message: RedditMessage) -> anyhow::Result<()> {
        let submission = match self.client.get_submission_by_link(message.body()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse or fetch post to redo: {:?} {e:?}",
                    message.body()
                );
                message.reply("Failed to parse or fetch which submission you meant.")?;
                return Ok(());
            }
        };

        let name = LowercaseString::new(submission.subreddit());

        let subconf = self.subreddits_config.get(&name);
        let modconf = subconf.map(|c| c.moderate.as_ref()).flatten();
        let flairconf = subconf.map(|c| &c.flairs);

        Self::check_post(
            &mut self.webhook,
            &self.analzyers,
            &mut self.imgur,
            modconf,
            flairconf,
            &self.templates,
            false,
            self.dry_run,
            submission,
        )?;
        Ok(())
    }

    fn check_inbox(&mut self) -> anyhow::Result<()> {
        let inbox = self.client.unread()?;
        for item in inbox {
            let subject = if let Some(stripped) = item.subject().strip_prefix("[dev-only]") {
                if is_debug() {
                    stripped.trim_start()
                } else {
                    continue;
                }
            } else {
                item.subject()
            };

            println!(
                "Saw inbox {:?} from /u/{}",
                subject,
                item.author()
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("no author")
            );

            item.mark_read()?;

            let author = match item.author() {
                Some(s) => s.as_str(),
                None => "",
            };

            if author == "AutoModerator" {
                continue;
            }

            if let Some(webhook) = &mut self.webhook {
                let inbox = create_inbox_message(&item);
                webhook.send(&inbox)?;
            }

            if subject == "test" {
                self.run_inbox_test(&item)?;
            } else if subject == "redo" {
                if let Some(admin) = self.admin.as_ref() {
                    if author == admin {
                        self.redo_from_message(item)?;
                    }
                } else {
                    eprintln!("  User attempted unauthorized redo");
                }
            } else if author == "" {
                if let Some(subreddit) = subject.strip_prefix("invitation to moderate /r/") {
                    let sub = self.client.subreddit(subreddit);
                    if let Err(e) = sub.accept_moderator_invite() {
                        println!("Unable to accept mod: {e:?}");
                    }
                }
            }
        }

        Ok(())
    }

    fn check_post(
        webhook: &mut Option<WebhookClient>,
        analzyers: &[Analyzer],
        imgur: &mut Option<ImgurClient>,
        modconf: Option<&SubredditModerateConfig>,
        flairs: Option<&SubredditFlairConfig>,
        templates: &Tera,
        has_seen: bool,
        dry_run: bool,
        post: Submission,
    ) -> anyhow::Result<()> {
        if let Some(flairs) = flairs {
            Self::check_post_flairs(dry_run, &post, webhook, flairs)?;
        }

        if has_seen {
            return Ok(());
        }

        let (ctx, warnings) = tryw!(context::Context::from_submission(&post), Result::Err);
        if warnings.len() > 0 {
            Self::_send_warnings(
                webhook,
                warnings,
                format!("Warnings with post {:?}, {:?}", post.id(), post.permalink()),
            )?;
        }

        let result = match analysis::get_best_analysis(&ctx, analzyers) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error whilst analyising {}: {err:?}", post.id());
                if let Some(webhook) = webhook {
                    let msg = create_error_processing_post(&post);
                    webhook.send(&msg)?;
                }
                return Ok(());
            }
        };

        if let Some((detection, detected)) = result {
            println!(
                "Triggered on post {:?} by /u/{}",
                post.title(),
                post.author()
            );

            let mut template_context = tera::Context::new();

            let is_mod = match (modconf, detected.remove) {
                (Some(modconf), true) if post.moderation().is_some() => {
                    template_context.insert("removal_reason", &modconf.removal_reason);
                    true
                }
                _ => {
                    println!(
                        "Not modding post (mod config? {}, should remove = {} and can_mod is {}",
                        modconf.is_some(),
                        detected.remove,
                        post.moderation().is_some()
                    );
                    false
                }
            };

            let imgur_link = match (ctx.images.len() > 0, imgur.as_mut()) {
                (true, Some(imgur)) => {
                    match imgur::upload_images(imgur, &ctx, &detection, detected) {
                        Ok(album) => {
                            let url = format!("https://imgur.com/a/{}", album.id);
                            template_context.insert("imgur_url", &url);
                            Some(url)
                        }
                        Err(e) => {
                            let msg = create_generic_error_message("Uploading to imgur", e);
                            if let Some(webhook) = webhook {
                                let _ = webhook.send(&msg);
                            }
                            None
                        }
                    }
                }
                (_, _) => None,
            };

            let template = templates
                .render(&detected.template, &template_context)
                .with_context(|| format!("rendering to template {:?}", detected.template))?;

            if !dry_run {
                let own_comment = post
                    .comment(&template)
                    .with_context(|| format!("reply to {:?}", post.name()))?;

                if is_mod {
                    post.remove(false)?;
                    own_comment.distinguish(Distinguish::Moderator, true)?;
                } else if detected.report {
                    post.report("Appears to be a common repost")
                        .with_context(|| format!("report {:?}", post.name()))?;
                }

                if let Some(webhook) = webhook {
                    let msg = create_detection_message(&post, &detection, detected, imgur_link);
                    webhook.send(&msg).context("send detection webhook")?;
                }
            }
        }
        Ok(())
    }

    fn check_subreddits(&mut self) -> anyhow::Result<()> {
        for subreddit in self.subreddits.iter_mut() {
            if subreddit.status_only {
                continue;
            }
            for post in subreddit.newest_unseen().context("get newest unseen")? {
                let is_removed = post.moderation().map(|m| m.removed).unwrap_or_default();
                if is_removed {
                    continue;
                }

                let has_seen = subreddit.is_seen(&post);

                if post.has_unknown_media() {
                    continue;
                } else if !has_seen {
                    subreddit.set_seen(&post);
                    println!(
                        "Saw {:?} {:?} by /u/{}",
                        post.name(),
                        post.title(),
                        post.author(),
                    );
                }

                let subconf = self.subreddits_config.get(subreddit.name());
                let modconf = subconf.map(|c| c.moderate.as_ref()).flatten();
                let flairconf = subconf.map(|c| &c.flairs);

                Self::check_post(
                    &mut self.webhook,
                    &self.analzyers,
                    &mut self.imgur,
                    modconf,
                    flairconf,
                    &self.templates,
                    has_seen,
                    self.dry_run,
                    post,
                )?;
            }
        }
        Ok(())
    }

    fn update_status_with(
        &mut self,
        mut cached: CachedIncidentSubmissions,
        is_summary: bool,
    ) -> anyhow::Result<()> {
        for subreddit in &mut self.subreddits {
            if let Some(config) = self.subreddits_config.get_status(subreddit.name()) {
                subreddit
                    .update_status(&self.client, &self.status, &mut cached, is_summary, config)
                    .with_context(|| format!("check status for /r/{}", subreddit.name()))?;
            }
        }

        Ok(())
    }

    fn check_status(&mut self) -> anyhow::Result<()> {
        let summary = self.status.get_summary()?;
        println!(
            "Status is {:?}, with {} incidents",
            summary.status.indicator,
            summary.incidents.len()
        );

        let summary = CachedIncidentSubmissions::new(summary.incidents);

        self.update_status_with(summary, true)
    }

    fn check_own_comments(&mut self) -> anyhow::Result<()> {
        for comment in self.client.comments(None)? {
            if !comment.score_hidden() && comment.score() < 0 {
                println!(
                    "Removing downvoted {:?} on {:?} by /u/{}",
                    comment.name(),
                    comment.link_title(),
                    comment.link_author()
                );
                comment.delete()?;
                if let Some(webhook) = &mut self.webhook {
                    let message = create_deleted_downvoted_comment(&comment);
                    webhook.send(&message)?;
                }
            }
        }

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let (tx, rx) = mpsc::channel();

        if let Some(addr) = &self.status_webhook {
            println!("Starting status webhook at {addr}");
            crate::reddit::status_tracker::start_webhook_listener_thread(tx, &addr);
        }

        loop {
            match self.ratelimit.get() {
                ratelimiter::Rate::NoneReadyFor(dur) => match rx.recv_timeout(dur) {
                    Ok(event) => match event {
                        status_tracker::WebhookEvent::IncidentUpdate(incident) => {
                            println!("[status-recv] got incident webhook");
                            let incident = *incident;
                            let cache = CachedIncidentSubmissions::new(vec![incident]);
                            self.update_status_with(cache, false)?;
                        }
                        _ => {
                            println!("[status-recv] got unknown webhook, checking status");
                            self.check_status()?;
                            self.ratelimit.set_status();
                        }
                    },
                    Err(RecvTimeoutError::Disconnected) => bail!("status webhook disconnected"),
                    Err(RecvTimeoutError::Timeout) => continue,
                },
                ratelimiter::Rate::InboxReady => {
                    println!("Checking inbox");
                    self.check_inbox().context("check inbox")?;
                    self.ratelimit.set_inbox();
                }
                ratelimiter::Rate::SubredditsReady => {
                    println!("Checking subreddits");
                    self.check_subreddits().context("check subreddits")?;
                    self.ratelimit.set_subreddits();
                }
                ratelimiter::Rate::StatusReady => {
                    println!("Checking status");
                    self.check_status().context("check status")?;
                    self.ratelimit.set_status();
                }
                ratelimiter::Rate::DownvotesReady => {
                    println!("Checking for downvoted comments");
                    self.check_own_comments().context("check own comments")?;
                    self.ratelimit.set_downvotes();
                }
            }
        }
    }

    pub fn send_webhook(&mut self, message: &DiscordMessage) -> anyhow::Result<()> {
        if let Some(webhook) = &mut self.webhook {
            webhook.send(message)?;
        }
        Ok(())
    }
}
