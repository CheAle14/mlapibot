use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::Context;
use roux::{builders::submission::SubmissionSubmitBuilder, Reddit};
use status_tracker::CachedSummary;
use statuspage::{incident::IncidentImpact, StatusClient};
use subreddit::Subreddit;
use tera::Tera;

use crate::{
    analysis::{self, Analyzer},
    context,
    imgur::{self, ImgurClient},
    webhook::{
        create_detection_message, create_error_processing_message, create_inbox_message,
        WebhookClient,
    },
    RedditInfo,
};

mod ratelimiter;
mod seen_tracker;
mod status_tracker;
mod subreddit;

pub struct RedditClient<'a> {
    data_dir: PathBuf,
    analzyers: &'a [Analyzer],
    me: roux::Me,
    subreddits: Vec<Subreddit>,
    ratelimit: ratelimiter::Ratelimiter,
    templates: Tera,
    webhook: Option<WebhookClient>,
    imgur: Option<ImgurClient>,
    status: StatusClient,
    status_filter: HashMap<String, IncidentImpact>,
}

impl<'a> RedditClient<'a> {
    const USER_AGENT: &'static str = "rust-mlapibot-ocr by /u/DarkOverLordCO";

    pub fn new(analzyers: &'a [Analyzer], args: &RedditInfo) -> anyhow::Result<Self> {
        let templates_path = args.data_dir.join("templates").join("*.md");
        let templates = Tera::new(templates_path.as_os_str().to_str().unwrap())?;
        let found: Vec<_> = templates.get_template_names().collect();
        assert!(found.len() > 0);
        let credentials = args.get_credentials()?;

        let me = Reddit::new(
            Self::USER_AGENT,
            &credentials.client_id,
            &credentials.client_secret,
        )
        .username(&credentials.username)
        .password(&credentials.password)
        .login()?;

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
            .map(|&name| Subreddit::new(args, roux::Subreddit::new_oauth(&name, &me.client)))
            .collect();

        println!(
            "Logged in as /u/{}; monitoring {} and sending status information to {} subreddits",
            credentials.username,
            args.subreddits.len(),
            status_filter.len()
        );

        for subreddit in &subreddits {
            if !status_filter.contains_key(subreddit.name()) {
                status_filter.insert(subreddit.name().to_owned(), IncidentImpact::Major);
            }
        }

        Ok(Self {
            me,
            subreddits,
            analzyers,
            ratelimit: ratelimiter::Ratelimiter::new(),
            data_dir: args.scratch_dir.clone(),
            templates,
            webhook,
            imgur,
            status,
            status_filter,
        })
    }

    fn check_inbox(&mut self) -> anyhow::Result<()> {
        let inbox = self.me.unread()?;
        for item in inbox.data.children {
            println!(
                "Saw inbox {} from {}",
                item.data.subject,
                item.data
                    .author
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("no author")
            );
            self.me.mark_read(&item.data.name)?;
            if let Some(webhook) = &mut self.webhook {
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
            for post in subreddit.newest_unseen()? {
                let ctx = context::Context::from_submission(&post)?;
                let result = match analysis::get_best_analysis(&ctx, &self.analzyers) {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error whilst analyising {}: {err:?}", post.id);
                        if let Some(webhook) = &mut self.webhook {
                            let msg = create_error_processing_message(&post);
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
                        .render(&detected.template, &template_context)?;

                    self.me.comment(&template, &post.name)?;

                    if detected.report {
                        self.me
                            .report(&post.name, "Appears to be a common repost")?;
                    }

                    if let Some(webhook) = &mut self.webhook {
                        let msg = create_detection_message(&post, &detection, detected, imgur_link);
                        webhook.send(&msg)?;
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
                .update_status(&self.me, &self.status, &mut summary, level)
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
}
