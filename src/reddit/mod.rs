use std::path::PathBuf;

use anyhow::Context;
use roux::{submission::SubmissionData, Reddit};
use seen_tracker::SeenTracker;
use subreddit::Subreddit;
use tera::Tera;

use crate::{
    analysis::{self, Analyzer, Detection},
    context,
    webhook::{create_detection_message, Message, MessageEmbed, MessageEmbedAuthor, WebhookClient},
    RedditInfo,
};

mod ratelimiter;
mod seen_tracker;
mod subreddit;

pub struct RedditClient<'a> {
    data_dir: PathBuf,
    analzyers: &'a [Analyzer],
    me: roux::Me,
    subreddits: Vec<Subreddit>,
    ratelimit: ratelimiter::Ratelimiter,
    templates: Tera,
    webhook: Option<WebhookClient>,
}

impl<'a> RedditClient<'a> {
    const USER_AGENT: &str = "rust-mlapibot-ocr by /u/DarkOverLordCO";

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

        println!(
            "Logged in as /u/{}; monitoring {} subreddits",
            credentials.username,
            args.subreddits.len()
        );

        let subreddits = args
            .subreddits
            .iter()
            .map(|name| Subreddit::new(args, roux::Subreddit::new_oauth(&name, &me.client)))
            .collect();

        let webhook = credentials
            .webhook_url
            .as_ref()
            .map(|url| WebhookClient::new(url))
            .transpose()?;

        Ok(Self {
            me,
            subreddits,
            analzyers,
            ratelimit: ratelimiter::Ratelimiter::new(),
            data_dir: args.scratch_dir.clone(),
            templates,
            webhook,
        })
    }

    fn check_inbox(&mut self) -> anyhow::Result<()> {
        let inbox = self.me.unread()?;
        for item in inbox.data.children {
            println!(
                "Saw inbox {} from {}",
                item.data.subject,
                item.data.author.unwrap_or(String::from("no author"))
            );
        }

        Ok(())
    }

    fn check_subreddits(&mut self) -> anyhow::Result<()> {
        for subreddit in self.subreddits.iter_mut() {
            for post in subreddit.newest_unseen()? {
                let ctx = context::Context::from_submission(&post)?;
                let result = match analysis::get_best_analysis(&ctx, &self.analzyers) {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error whilst analyising {}: {err:?}", post.id);
                        return Ok(());
                    }
                };

                if let Some((detection, detected)) = result {
                    println!("Triggered on post {:?} by /u/{}", post.title, post.author);
                    let md = detection.get_markdown(&ctx)?;
                    let md = md.join("\n\n---\n\n");

                    let template_context = tera::Context::new();
                    let template = self
                        .templates
                        .render(&detected.template, &template_context)?;

                    self.me.comment(&template, &post.name)?;

                    if let Some(webhook) = &mut self.webhook {
                        let msg = create_detection_message(&post, &detection, detected);
                        webhook.send(&msg)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.ratelimit.get() {
                ratelimiter::Rate::NoneReadyFor(dur) => {
                    println!("Sleeping for {dur:?}");
                    std::thread::sleep(dur)
                }
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
            }
        }
    }
}
