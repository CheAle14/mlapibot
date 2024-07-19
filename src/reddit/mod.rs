use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::Context;
use roux::client::{OAuthClient, RedditClient as RouxRedditClient};
use roux::inbox::InboxData;
use status_tracker::CachedSummary;
use statuspage::{incident::IncidentImpact, StatusClient};
use subreddit::Subreddit;
use tera::Tera;

use crate::{
    analysis::{self, get_best_analysis, Analyzer},
    context,
    imgur::{self, ImgurClient},
    webhook::{
        create_detection_message, create_error_processing_message, create_error_processing_post,
        create_inbox_message, Message, WebhookClient,
    },
    RedditInfo,
};

mod ratelimiter;
mod seen_tracker;
mod status_tracker;
mod subreddit;

pub type RouxClient = roux::client::AuthedClient;

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
    status_filter: HashMap<String, IncidentImpact>,
    dry_run: bool,
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

        let mut status_filter = args.get_status_levels()?;

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

        for subreddit in &subreddits {
            if !status_filter.contains_key(subreddit.name()) {
                status_filter.insert(subreddit.name().to_owned(), IncidentImpact::Major);
            }
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
            status_filter,
            dry_run: args.dry_run,
        })
    }

    fn run_inbox_test(&mut self, message: &InboxData) -> anyhow::Result<()> {
        let ctx = crate::context::Context::from_direct_message(message)?;

        match get_best_analysis(&ctx, &self.analzyers) {
            Ok(Some((detection, detected))) => {
                let text = detection.get_markdown(&ctx)?;
                let text = text.join("\n\n\n> ");
                let s = format!("Detected {:?}. Full text:\r\n\r\n> {text}", detected.name);
                self.client.comment(&s, &message.name)?;
            }
            Ok(None) => {
                let mut text = String::from("No scams were detected, text was:\r\n\r\n");
                for img in &ctx.images {
                    text.push_str("> ");
                    text.push_str(&img.full_text());
                    text.push_str("\n\n\n");
                }
                self.client.comment(&text, &message.name)?;
            }
            Err(err) => {
                eprintln!(
                    "Error whilst analyising message {:?}: {err:?}",
                    message.subject
                );
                if let Some(webhook) = &mut self.webhook {
                    let msg = create_error_processing_message(&message);
                    webhook.send(&msg)?;
                }
                self.client.comment(
                    "An internal error occured whilst attempting to process your request. Sorry!",
                    &message.name,
                )?;
            }
        };
        Ok(())
    }

    fn check_inbox(&mut self) -> anyhow::Result<()> {
        let inbox = self.client.unread()?;
        for item in inbox.data.children {
            println!(
                "Saw inbox {:?} from /u/{}",
                item.data.subject,
                item.data
                    .author
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("no author")
            );
            self.client.mark_read(&item.data.name)?;
            if item.data.subject == "test" {
                self.run_inbox_test(&item.data)?;
            } else if let Some(webhook) = &mut self.webhook {
                let inbox = create_inbox_message(&item.data);
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
                println!("Saw {:?} {:?} by /u/{}", post.name, post.title, post.author);
                let ctx = context::Context::from_submission(&post)?;
                let result = match analysis::get_best_analysis(&ctx, &self.analzyers) {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error whilst analyising {}: {err:?}", post.id);
                        if let Some(webhook) = &mut self.webhook {
                            let msg = create_error_processing_post(&post);
                            webhook.send(&msg)?;
                        }
                        continue;
                    }
                };

                if let Some((detection, detected)) = result {
                    println!("Triggered on post {:?} by /u/{}", post.title, post.author);

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
                        self.client
                            .comment(&template, &post.name)
                            .with_context(|| format!("reply to {:?}", post.name))?;

                        if detected.report {
                            self.client
                                .report(&post.name, "Appears to be a common repost")
                                .with_context(|| format!("report {:?}", post.name))?;
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

    fn check_status(&mut self) -> anyhow::Result<()> {
        let summary = self.status.get_summary()?;
        println!(
            "Status is {:?}, with {} incidents",
            summary.status.indicator,
            summary.incidents.len()
        );
        // TODO: only compute if any subreddit post needs updating
        let mut summary = CachedSummary::new(summary)?;
        for subreddit in &mut self.subreddits {
            let level = self.status_filter.get(subreddit.name()).unwrap();
            subreddit
                .update_status(&self.client, &self.status, &mut summary, level)
                .with_context(|| format!("check status for /r/{}", subreddit.name()))?;
        }

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.ratelimit.get() {
                ratelimiter::Rate::NoneReadyFor(dur) => std::thread::sleep(dur),
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
            }
        }
    }

    pub fn send_webhook(&mut self, message: &Message) -> anyhow::Result<()> {
        if let Some(webhook) = &mut self.webhook {
            webhook.send(message)?;
        }
        Ok(())
    }
}
