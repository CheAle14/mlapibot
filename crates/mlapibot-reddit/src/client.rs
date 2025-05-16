use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError},
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use mlapibot_common::{Cached, LowercaseString};
use mlapibot_datastore::MlapiDb;
use roux::{
    api::{Distinguished, ThingFullname, subreddit::ModActionType},
    client::{OAuthClient, RedditClient as RouxRedditClient},
    models::Distinguish,
};
use statuspage::{StatusClient, component::Component, status::StatusIndicator};
use tera::Tera;

use mlapibot_analysis::{ContextWarning, analzyer::Analyzer};
use mlapibot_imgur::ImgurClient;
use mlapibot_webhook::{
    WebhookClient, create_generic_error_message, create_multiple_error_message,
};

use super::{RedditMessage, RouxClient, Submission};

use crate::{
    config::{RedditCredentials, SubredditModerateConfig, SubredditsConfig},
    exts::{DetectionExt, SubmissionExt},
    flairs::{PostFlairCache, SubredditFlairConfig},
    ratelimiter::Ratelimiter,
    status_tracker::{CachedIncidentSubmissions, WebhookEvent},
    subreddit::Subreddit,
    webhook::{
        create_deleted_downvoted_comment, create_detection_message,
        create_error_processing_message, create_error_processing_post, create_inbox_message,
        create_moderator_downvoted_comment,
    },
};

pub struct RedditClient<'a> {
    // data_dir: PathBuf,
    db: MlapiDb,
    analzyers: &'a [Analyzer],
    own_name: String,
    client: RouxClient,
    subreddits: Vec<Subreddit>,
    templates: Tera,
    webhook: Option<WebhookClient>,
    imgur: Option<ImgurClient>,
    status: StatusClient,
    last_status: StatusIndicator,
    subreddits_config: SubredditsConfig,
    dry_run: bool,
    status_webhook: Option<String>,
    #[allow(unused)]
    admin: Option<String>,
    flair_cache: PostFlairCache,
    cached_status_components: StatusComponentCache,
    debug: bool,
}

pub type StatusComponentCache =
    Cached<HashMap<String, Component>, StatusClient, statuspage::error::Error>;

impl<'a> RedditClient<'a> {
    const USER_AGENT: &'static str = "rust-mlapibot-ocr by /u/DarkOverLordCO";

    pub fn new(
        analzyers: &'a [Analyzer],
        data_dir: PathBuf,
        database_path: PathBuf,
        subreddits: Vec<LowercaseString>,
        dry_run: bool,
        status_webhook: Option<String>,
        admin: Option<String>,
        credentials: RedditCredentials,
        subreddits_config: SubredditsConfig,
        debug: bool,
    ) -> anyhow::Result<Self> {
        let templates_path = data_dir.join("templates").join("*.md");
        let templates = Tera::new(templates_path.as_os_str().to_str().unwrap())?;
        let found: Vec<_> = templates.get_template_names().collect();
        assert!(found.len() > 0);

        let db = MlapiDb::new(database_path).context("initialize db")?;

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
            .map(|creds| ImgurClient::new(&creds.imgur_client_id))
            .transpose()?;

        let status = StatusClient::new("https://discordstatus.com")?;

        let mut subreddit_names = HashSet::new();
        for sub in &subreddits {
            subreddit_names.insert(sub.clone());
        }
        for key in subreddits_config.keys() {
            subreddit_names.insert(key.clone());
        }

        let subreddits: Result<Vec<Subreddit>, _> = subreddit_names
            .into_iter()
            .map(|name| {
                let status_only = subreddits.iter().find(|&s| s == &name).is_none();

                Subreddit::new(status_only, client.subreddit(name.as_str()), name)
            })
            .collect();

        let subreddits = subreddits?;

        let cached_status_components = Cached::new(Duration::from_secs(600), &status, |client| {
            let mut map = HashMap::new();
            let vec = client.get_components()?;

            for item in vec {
                map.insert(item.id.clone(), item);
            }

            Ok(map)
        })?;

        println!(
            "Logged in as /u/{}; monitoring {} with {} total known subreddits in {}",
            credentials.username,
            subreddits.len(),
            subreddits_config.len(),
            if debug { "debug mode" } else { "release mode" }
        );

        if dry_run {
            println!("Running in dry-run mode.");
        }

        Ok(Self {
            db,
            own_name: credentials.username,
            client,
            subreddits,
            analzyers,
            // data_dir: scratch_dir,
            templates,
            webhook,
            imgur,
            status,
            subreddits_config,
            dry_run: dry_run,
            status_webhook: status_webhook,
            admin: admin,
            flair_cache: PostFlairCache::default(),
            last_status: StatusIndicator::None,
            cached_status_components,
            debug,
        })
    }

    fn _send_warnings(
        webhook: &mut Option<WebhookClient>,
        warnings: Vec<ContextWarning>,
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
        warnings: Vec<ContextWarning>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        Self::_send_warnings(&mut self.webhook, warnings, context)
    }

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

    fn try_run_media_count(&mut self, author: &str, message: &RedditMessage) -> anyhow::Result<()> {
        use std::fmt::Write;

        let sub = self.client.subreddit(message.body());

        let our_sub = self.subreddits.iter_mut().find(|s| s.name() == &sub.name);
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
        author: &str,
        message: &RedditMessage,
    ) -> anyhow::Result<()> {
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
            return Ok(());
        }

        let name = LowercaseString::new(submission.subreddit());
        let Some(subreddit) = self.subreddits.iter_mut().find(|s| s.name() == &name) else {
            message.reply("That subreddit is not monitored")?;
            return Ok(());
        };

        let subconf = self.subreddits_config.get(&name);
        let modconf = subconf.map(|c| c.moderate.as_ref()).flatten();
        let flairconf = subconf.map(|c| &c.flairs);

        Self::check_post(
            &self.db,
            &mut self.webhook,
            &self.analzyers,
            &mut self.imgur,
            modconf,
            flairconf,
            &mut self.flair_cache,
            &self.templates,
            false,
            self.dry_run,
            subreddit,
            submission,
        )?;
        Ok(())
    }

    fn send_removal_reasons(&mut self, message: &RedditMessage) -> anyhow::Result<()> {
        use std::fmt::Write;

        let subreddit = message.body().trim().trim_start_matches("/r/");
        let mut sending = format!("Removal reasons for /r/{subreddit}:\n");
        let subreddit = self.client.subreddit(subreddit);

        let reasons = subreddit.list_removal_reasons()?;

        for id in reasons.order {
            let _ = match reasons.data.get(&id) {
                Some(reason) => write!(sending, "- {}: {}", reason.id, reason.message),
                None => write!(sending, "- {id}: <not found>"),
            };
        }

        message.reply(&sending)?;

        Ok(())
    }

    fn check_inbox(&mut self) -> anyhow::Result<Duration> {
        let inbox = self.client.unread()?;
        for item in inbox {
            let subject = if let Some(stripped) = item.subject().strip_prefix("[dev-only]") {
                if self.debug {
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

            if self.last_status != StatusIndicator::Critical {
                if let Some(webhook) = &mut self.webhook {
                    let inbox = create_inbox_message(&item);
                    webhook.send(&inbox)?;
                }
            }

            if subject == "test" {
                self.run_inbox_test(&item)?;
            } else if subject == "redo" {
                self.try_redo_from_message(author, &item)?;
            } else if subject == "media" {
                self.try_run_media_count(author, &item)?;
            } else if subject == "removal_reasons" {
                self.send_removal_reasons(&item)?;
            } else if author == "" {
                if let Some(subreddit) = subject.strip_prefix("invitation to moderate /r/") {
                    let sub = self.client.subreddit(subreddit);
                    if let Err(e) = sub.accept_moderator_invite() {
                        println!("Unable to accept mod: {e:?}");
                    }
                }
            }
        }

        Ok(Duration::from_secs(15))
    }

    fn check_post(
        db: &MlapiDb,
        webhook: &mut Option<WebhookClient>,
        analzyers: &[Analyzer],
        imgur: &mut Option<ImgurClient>,
        modconf: Option<&SubredditModerateConfig>,
        flairs: Option<&SubredditFlairConfig>,
        flair_cache: &mut PostFlairCache,
        templates: &Tera,
        has_seen: bool,
        dry_run: bool,
        subreddit: &mut Subreddit,
        post: Submission,
    ) -> anyhow::Result<()> {
        if let Some(flairs) = flairs {
            Self::check_post_flairs(dry_run, subreddit, &post, webhook, flairs, flair_cache)?;
        }

        if has_seen {
            return Ok(());
        }

        let mut warnings = Vec::new();
        let ctx = mlapibot_analysis::Context::new_submission(
            post.get_misc_links().into_iter(),
            post.title(),
            post.selftext(),
            &mut warnings,
        )?;

        if warnings.len() > 0 {
            Self::_send_warnings(
                webhook,
                warnings,
                format!("Warnings with post {:?}, {:?}", post.id(), post.permalink()),
            )?;
        }

        let result = match mlapibot_analysis::get_best_analysis(&ctx, analzyers) {
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
                    let reason_id = modconf
                        .removal_reasons
                        .get(&detected.name)
                        .map(String::as_str)
                        .unwrap_or_else(|| &modconf.default_removal_reason);

                    let reason = subreddit
                        .get_removal_reason(reason_id)?
                        .map(|r| r.message.as_str())
                        .unwrap_or("<error: removal reason not found>");

                    template_context.insert("removal_reason", reason);
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

            let imgur_link = match (
                detected.template.name().is_some(), // no point uploading images if we aren't replying
                ctx.images.len() > 0,
                imgur.as_mut(),
            ) {
                (true, true, Some(imgur)) => {
                    match mlapibot_imgur::upload_images(imgur, ctx.images.iter(), |idx| {
                        detection
                            .images
                            .get(&idx)
                            .map(|d| ctx.images[idx].get_trigger_words_image(d))
                            .flatten()
                    }) {
                        Ok(album) => {
                            let url = format!("https://imgur.com/a/{}", album.id);
                            template_context.insert("imgur_url", &url);
                            Some(url)
                        }
                        Err(e) => {
                            let msg = create_generic_error_message(
                                "Uploading to imgur",
                                format!("{e:?}"),
                            );
                            if let Some(webhook) = webhook {
                                let _ = webhook.send(&msg);
                            }
                            None
                        }
                    }
                }
                (_, _, _) => None,
            };

            let (reply, removed, reported) = if !dry_run {
                let own_comment = match detected.template.name() {
                    Some(text) => {
                        let template =
                            templates.render(text, &template_context).with_context(|| {
                                format!("rendering to template {:?}", detected.template)
                            })?;

                        let comment = post
                            .comment(&template)
                            .with_context(|| format!("reply to {:?}", post.name()))?;

                        Some(comment)
                    }
                    None => None,
                };

                if is_mod {
                    post.remove(false)?;

                    if let Some(own_comment) = &own_comment {
                        own_comment.distinguish(Distinguish::Moderator, true)?;
                    }
                } else if detected.report {
                    post.report(&format!(
                        "Appears to be a common repost ({})",
                        detected.name
                    ))
                    .with_context(|| format!("report {:?}", post.name()))?;
                }

                if let Some(webhook) = webhook {
                    let msg = create_detection_message(&post, &detection, detected, imgur_link);
                    webhook.send(&msg).context("send detection webhook")?;
                }

                (
                    own_comment.map(|c| c.name().full().to_string()),
                    is_mod,
                    !is_mod && detected.report,
                )
            } else {
                (None, false, false)
            };

            db.set_analyzed(
                post.name().full(),
                &detected.name,
                reply.as_ref().map(|s| s.as_str()),
                reported,
                removed,
            )?;
        } else {
            db.set_ignored(post.name().full())?;
        }
        Ok(())
    }

    fn check_subreddits(&mut self) -> anyhow::Result<Duration> {
        for subreddit in self.subreddits.iter_mut() {
            if subreddit.status_only {
                continue;
            }
            for post in subreddit.newest_unseen().context("get newest unseen")? {
                if post.author() == &self.own_name {
                    continue;
                }

                let is_removed = post.moderation().map(|m| m.removed).unwrap_or_default();
                if is_removed {
                    continue;
                }

                let has_seen = self
                    .db
                    .has_seen(post.name().full())
                    .context("lookup seen")?;

                if post.has_unknown_media() {
                    continue;
                } else if !has_seen {
                    self.db
                        .set_seen(subreddit.name().as_str(), post.name().full())
                        .context("add monitor")?;

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
                    &self.db,
                    &mut self.webhook,
                    &self.analzyers,
                    &mut self.imgur,
                    modconf,
                    flairconf,
                    &mut self.flair_cache,
                    &self.templates,
                    has_seen,
                    self.dry_run,
                    subreddit,
                    post,
                )?;
            }
        }
        Ok(Duration::from_secs(15))
    }

    fn update_status_with(
        &mut self,
        mut cached: CachedIncidentSubmissions,
        is_summary: bool,
    ) -> anyhow::Result<()> {
        for subreddit in &mut self.subreddits {
            if let Some(config) = self.subreddits_config.get_status(subreddit.name()) {
                subreddit
                    .update_status(
                        &self.db,
                        &self.client,
                        &self.status,
                        &mut cached,
                        &mut self.cached_status_components,
                        is_summary,
                        config,
                    )
                    .with_context(|| format!("check status for /r/{}", subreddit.name()))?;
            }
        }

        Ok(())
    }

    fn check_status(&mut self) -> anyhow::Result<Duration> {
        let summary = self.status.get_summary()?;
        self.last_status = summary.status.indicator;
        println!(
            "Status is {:?}, with {} incidents",
            summary.status.indicator,
            summary.incidents.len()
        );

        let summary = CachedIncidentSubmissions::new(summary.incidents);

        self.update_status_with(summary, true)?;

        Ok(Duration::from_secs(5 * 60))
    }

    fn check_own_comments(&mut self) -> anyhow::Result<Duration> {
        let comments = self.client.comments(None)?;

        for comment in comments {
            if !comment.score_hidden() && comment.score() < 0 {
                if matches!(comment.distinguished(), Distinguished::None) {
                    println!(
                        "Removing downvoted {:?} on {:?} by /u/{}",
                        comment.name(),
                        comment.link_title(),
                        comment.link_author()
                    );
                    comment.delete()?;
                    self.db.set_mistaken(comment.name().full())?;
                    if let Some(webhook) = &mut self.webhook {
                        let message = create_deleted_downvoted_comment(&comment);
                        webhook.send(&message)?;
                    }
                } else {
                    println!(
                        "NOT removing downvoted {:?} on {:?} by /u/{}",
                        comment.name(),
                        comment.link_title(),
                        comment.link_author()
                    );
                    if let Some(webhook) = &mut self.webhook {
                        let message = create_moderator_downvoted_comment(&comment);
                        webhook.send(&message)?;
                    }
                }
            }
        }

        Ok(Duration::from_secs(15))
    }

    fn handle_webhook_event(
        &mut self,
        event: WebhookEvent,
        ratelimiter: &mut Ratelimiter<Self>,
    ) -> anyhow::Result<()> {
        match event {
            crate::status_tracker::WebhookEvent::IncidentUpdate(incident) => {
                println!("[status-recv] got incident webhook");
                let incident = *incident;
                let cache = CachedIncidentSubmissions::new(vec![incident]);
                self.update_status_with(cache, false)?;
            }
            _ => {
                println!("[status-recv] got unknown webhook, scheduling status check to run");
                ratelimiter.run_immediately("check_status");
            }
        };
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let (tx, rx) = mpsc::channel();

        if let Some(addr) = &self.status_webhook {
            println!("Starting status webhook at {addr}");
            crate::status_tracker::start_webhook_listener_thread(tx, &addr);
        }

        let mut ratelimiter = Ratelimiter::new();
        ratelimiter.push("check_inbox", Self::check_inbox);
        ratelimiter.push("check_subreddits", Self::check_subreddits);
        ratelimiter.push("check_status", Self::check_status);
        ratelimiter.push("check_own_comments", Self::check_own_comments);

        loop {
            while let Ok(event) = rx.try_recv() {
                self.handle_webhook_event(event, &mut ratelimiter)?;
            }

            let now = Instant::now();
            let next = ratelimiter.run(self, now);

            match rx.recv_timeout(next - now) {
                Ok(event) => self.handle_webhook_event(event, &mut ratelimiter)?,
                Err(RecvTimeoutError::Disconnected) => bail!("status webhook disconnected"),
                Err(RecvTimeoutError::Timeout) => continue,
            }
        }
    }

    pub fn send_webhook(&mut self, message: &mlapibot_webhook::Message) -> anyhow::Result<()> {
        if let Some(webhook) = &mut self.webhook {
            webhook.send(message)?;
        }
        Ok(())
    }
}
