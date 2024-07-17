use std::path::PathBuf;

use anyhow::Context;
use roux::Reddit;
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

        let imgur = credentials
            .imgur_credentials
            .as_ref()
            .map(|creds| ImgurClient::new(creds))
            .transpose()?;

        Ok(Self {
            me,
            subreddits,
            analzyers,
            ratelimit: ratelimiter::Ratelimiter::new(),
            data_dir: args.scratch_dir.clone(),
            templates,
            webhook,
            imgur,
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

                    if let Some(webhook) = &mut self.webhook {
                        let msg = create_detection_message(&post, &detection, detected, imgur_link);
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
                ratelimiter::Rate::NoneReadyFor(dur) => std::thread::sleep(dur),
                ratelimiter::Rate::InboxReady => {
                    println!("{:?}: Checking inbox", chrono::Utc::now());
                    self.check_inbox().context("check inbox")?;
                    self.ratelimit.set_inbox();
                }
                ratelimiter::Rate::SubredditsReady => {
                    println!("{:?}: Checking subreddits", chrono::Utc::now());
                    self.check_subreddits().context("check subreddits")?;
                    self.ratelimit.set_subreddits();
                }
            }
        }
    }
}
