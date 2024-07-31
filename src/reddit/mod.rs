use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError},
};

use anyhow::{bail, Context};
use roux::client::{OAuthClient, RedditClient as RouxRedditClient};
use status_tracker::CachedIncidentSubmissions;
use statuspage::StatusClient;
use subreddit::Subreddit;
use tera::Tera;

use crate::{
    analysis::{self, get_best_analysis, Analyzer},
    context,
    imgur::{self, ImgurClient},
    webhook::{
        create_deleted_downvoted_comment, create_detection_message,
        create_error_processing_message, create_error_processing_post, create_inbox_message,
        Message as DiscordMessage, WebhookClient,
    },
    RedditInfo, SubredditStatusConfig,
};

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
    status_config: HashMap<String, SubredditStatusConfig>,
    dry_run: bool,
    status_webhook: Option<String>,
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

        let status_filter = args.get_status_levels()?;

        let mut subreddit_names = HashSet::new();
        for sub in &args.subreddits {
            subreddit_names.insert(sub);
        }
        for key in status_filter.keys() {
            subreddit_names.insert(key);
        }

        let subreddits: Vec<Subreddit> = subreddit_names
            .iter()
            .map(|&name| Subreddit::new(args, client.subreddit(&name)))
            .collect();

        println!(
            "Logged in as /u/{}; monitoring {} and sending status information to {} subreddits",
            credentials.username,
            args.subreddits.len(),
            status_filter.len()
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
            status_config: status_filter,
            dry_run: args.dry_run,
            status_webhook: args.status_webhook.clone(),
        })
    }

    fn run_inbox_test(&mut self, message: &RedditMessage) -> anyhow::Result<()> {
        let ctx = crate::context::Context::from_direct_message(message)?;

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

    fn check_inbox(&mut self) -> anyhow::Result<()> {
        let inbox = self.client.unread()?;
        for item in inbox {
            println!(
                "Saw inbox {:?} from /u/{}",
                item.subject(),
                item.author()
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("no author")
            );
            item.mark_read()?;
            if item.subject() == "test" {
                self.run_inbox_test(&item)?;
            } else if let Some(webhook) = &mut self.webhook {
                let inbox = create_inbox_message(&item);
                webhook.send(&inbox)?;
            }
        }

        Ok(())
    }

    fn check_subreddits(&mut self) -> anyhow::Result<()> {
        for subreddit in self.subreddits.iter_mut() {
            if subreddit.status_only {
                continue;
            }
            for post in subreddit.newest_unseen().context("get netwest unseen")? {
                println!(
                    "Saw {:?} {:?} by /u/{}",
                    post.name(),
                    post.title(),
                    post.author()
                );
                subreddit.set_seen(&post);
                let ctx = context::Context::from_submission(&post)?;
                let result = match analysis::get_best_analysis(&ctx, &self.analzyers) {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error whilst analyising {}: {err:?}", post.id());
                        if let Some(webhook) = &mut self.webhook {
                            let msg = create_error_processing_post(&post);
                            webhook.send(&msg)?;
                        }
                        continue;
                    }
                };

                if let Some((detection, detected)) = result {
                    println!(
                        "Triggered on post {:?} by /u/{}",
                        post.title(),
                        post.author()
                    );

                    let mut template_context = tera::Context::new();

                    let imgur_link = match (ctx.images.len() > 0, self.imgur.as_mut()) {
                        (true, Some(imgur)) => {
                            let album = imgur::upload_images(imgur, &ctx, &detection, detected)
                                .context("uploading to imgur")?;
                            let url = format!("https://imgur.com/a/{}", album.id);
                            template_context.insert("imgur_url", &url);
                            Some(url)
                        }
                        (_, _) => None,
                    };

                    let template = self
                        .templates
                        .render(&detected.template, &template_context)
                        .with_context(|| {
                            format!("rendering to template {:?}", detected.template)
                        })?;

                    if !self.dry_run {
                        post.comment(&template)
                            .with_context(|| format!("reply to {:?}", post.name()))?;

                        if detected.report {
                            post.report("Appears to be a common repost")
                                .with_context(|| format!("report {:?}", post.name()))?;
                        }

                        if let Some(webhook) = &mut self.webhook {
                            let msg =
                                create_detection_message(&post, &detection, detected, imgur_link);
                            webhook.send(&msg).context("send detection webhook")?;
                        }
                    }
                }
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
            if let Some(config) = self.status_config.get(subreddit.name()) {
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
                            let incident = *incident;
                            let cache = CachedIncidentSubmissions::new(vec![incident]);
                            self.update_status_with(cache, false)?;
                        }
                        _ => {
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
