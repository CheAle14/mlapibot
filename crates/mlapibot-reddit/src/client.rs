use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError},
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use mlapibot_common::Cached;
use mlapibot_datastore::{MlapiDb, live_incident_posts::LiveIncidentPost};
use octocrab::OctocrabBuilder;
use roux::{
    api::Distinguished,
    client::{OAuthClient, RedditClient as RouxRedditClient},
    util::{SubmissionStream, now_utc},
};
use statuspage::{StatusClient, component::Component, incident::Incident, status::StatusIndicator};
use tera::Tera;

use mlapibot_analysis::{ContextWarning, analzyer::Analyzer};
use mlapibot_imgur::ImgurClient;
use mlapibot_webhook::{WebhookClient, create_multiple_error_message};

use super::{RouxClient, Submission};

use crate::{
    client::module::{
        InboxAction, InboxMsg, PostAction, RegisteredModule, SplitSubMask,
        post_flairs::PostFlairCache,
    },
    config::{GlobalSettings, SubredditsConfig},
    exts::SubmissionExt,
    ratelimiter::Ratelimiter,
    status_tracker::{IncidentWithLive, WebhookEvent, get_title, write_affected_components_list},
    subreddit::Subreddit,
    utils::BoO,
    webhook::{create_deleted_downvoted_comment, create_inbox_message},
};

pub mod module;

pub struct RedditClient<'a> {
    // data_dir: PathBuf,
    db: MlapiDb,
    analzyers: &'a [Analyzer],
    own_name: String,
    pub client: RouxClient,
    subreddits: Vec<Subreddit>,
    templates: Tera,
    webhook: Option<WebhookClient>,
    imgur: Option<ImgurClient>,
    github: Option<octocrab::Octocrab>,
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

    posts: roux::util::SubmissionStream<Submission>,

    modules: Vec<RegisteredModule>,
    // the subreddit posts/comments wanted by *any* module.
    subreddits_mask: SplitSubMask,
}

pub type StatusComponentCache =
    Cached<HashMap<String, Component>, StatusClient, statuspage::error::Error>;

macro_rules! make_view {
    ($self:ident) => {
        ModuleRedditClient {
            db: &$self.db,
            analzyers: &$self.analzyers,
            own_name: &$self.own_name,
            client: &$self.client,
            templates: &$self.templates,
            webhook: &mut $self.webhook,
            imgur: &mut $self.imgur,
            github: &mut $self.github,
            status: &$self.status,
            last_status: &$self.last_status,
            subreddits_config: &$self.subreddits_config,
            dry_run: $self.dry_run,
            status_webhook: &$self.status_webhook,
            admin: &$self.admin,
            flair_cache: &mut $self.flair_cache,
            cached_status_components: &$self.cached_status_components,
            debug: $self.debug,
        }
    };
}

impl<'a> RedditClient<'a> {
    const USER_AGENT: &'static str = "rust-mlapibot-ocr by /u/DarkOverLordCO";

    pub async fn new(
        analzyers: &'a [Analyzer],
        data_dir: PathBuf,
        database_path: PathBuf,
        dry_run: bool,
        status_webhook: Option<String>,
        admin: Option<String>,
        settings: GlobalSettings,
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
            &settings.reddit.client_id,
            &settings.reddit.client_secret,
        )
        .username(&settings.reddit.username)
        .password(&settings.reddit.password);

        let client = OAuthClient::new(config)?.login().await?;

        let webhook = settings
            .webhook_url
            .as_ref()
            .map(|url| WebhookClient::new(url))
            .transpose()?;

        let imgur = settings
            .imgur
            .as_ref()
            .map(|creds| ImgurClient::new(&creds.client_id))
            .transpose()?;

        let github = settings
            .github
            .as_ref()
            .map(|creds| {
                OctocrabBuilder::new()
                    .personal_token(creds.token.as_str())
                    .build()
            })
            .transpose()?;

        let status = StatusClient::new("https://discordstatus.com")?;

        let mut subreddit_names = HashSet::new();
        for key in subreddits_config.keys() {
            subreddit_names.insert(key.clone());
        }

        let subreddits: Result<Vec<Subreddit>, _> = subreddit_names
            .into_iter()
            .map(|name| Subreddit::new(client.subreddit(name.as_str()), name))
            .collect();

        let subreddits = subreddits?;

        let cached_status_components = Cached::new(Duration::from_secs(600), &status, |client| {
            Box::pin(async {
                let mut map = HashMap::new();
                let vec = client.get_components().await?;

                for item in vec {
                    map.insert(item.id.clone(), item);
                }

                Ok(map)
            })
        })
        .await?;

        println!(
            "Logged in as /u/{}; monitoring {} with {} total known subreddits in {}",
            settings.reddit.username,
            subreddits.len(),
            subreddits_config.len(),
            if debug { "debug mode" } else { "release mode" }
        );

        if dry_run {
            println!("Running in dry-run mode.");
        }

        let (subreddits_mask, modules) = Self::build_modules(&subreddits_config, &subreddits);

        let posts = SubmissionStream::new(
            25,
            subreddits.iter().enumerate().filter_map(|(idx, sub)| {
                if subreddits_mask.posts.is_set(idx) {
                    Some(sub.name().as_str())
                } else {
                    None
                }
            }),
        );

        Ok(Self {
            db,
            own_name: settings.reddit.username,
            client,
            subreddits,
            analzyers,
            // data_dir: scratch_dir,
            templates,
            webhook,
            imgur,
            github,
            status,
            subreddits_config,
            dry_run: dry_run,
            status_webhook: status_webhook,
            admin: admin,
            flair_cache: PostFlairCache::default(),
            last_status: StatusIndicator::None,
            cached_status_components,
            debug,
            posts,

            modules,
            subreddits_mask,
        })
    }

    async fn _send_warnings(
        webhook: Option<&mut WebhookClient>,
        warnings: Vec<ContextWarning>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        if warnings.len() > 0 {
            if let Some(webhook) = webhook {
                let message = create_multiple_error_message(context, warnings);
                webhook.send(&message).await?;
            }
        }

        Ok(())
    }

    async fn send_warnings(
        &mut self,
        warnings: Vec<ContextWarning>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        Self::_send_warnings(self.webhook.as_mut(), warnings, context).await
    }

    async fn check_inbox(&mut self) -> anyhow::Result<Duration> {
        let inbox = self.client.unread().await?;
        for item in inbox {
            let (msg, dev_only) = InboxMsg::new(&item);

            if dev_only != self.debug {
                continue;
            }

            println!("Saw inbox {:?} from /u/{}", msg.subject, msg.author,);

            item.mark_read().await?;

            if msg.author == "AutoModerator" {
                continue;
            }

            if self.last_status != StatusIndicator::Critical {
                if let Some(webhook) = &mut self.webhook {
                    let inbox = create_inbox_message(&item);
                    webhook.send(&inbox).await?;
                }
            }

            let mut view = make_view!(self);

            let mut actions = Vec::new();

            for registration in &mut self.modules {
                if !registration.module.wants().inbox() {
                    continue;
                }

                let name = registration.module.name();

                let action = registration
                    .module
                    .run_inbox(&mut view, &mut self.subreddits, &msg)
                    .await
                    .with_context(|| format!("{name}.run_inbox"))?;

                match action {
                    Some(action) => actions.push(action),
                    _ => (),
                }
            }

            for action in actions {
                match action {
                    InboxAction::RedoSub(post) => {
                        let Some(idx) = self
                            .subreddits
                            .iter_mut()
                            .position(|s| s.name() == post.subreddit())
                        else {
                            continue;
                        };

                        let subreddit = &mut self.subreddits[idx];

                        view.run_post(&mut self.modules, subreddit, idx, post, false)
                            .await?;
                    }
                    InboxAction::RedoMsg(sub, msg) => {
                        let Some(idx) = self
                            .subreddits
                            .iter_mut()
                            .position(|s| s.name() == sub.subreddit())
                        else {
                            continue;
                        };

                        for reg in &mut self.modules {
                            if !reg.module.wants().comments() || !reg.submask.comments.is_set(idx) {
                                continue;
                            }

                            reg.module
                                .run_comment(&mut view, &msg)
                                .await
                                .with_context(|| {
                                    format!(
                                        "redo run-comment /r/{}/{}/{}",
                                        sub.subreddit(),
                                        sub.id(),
                                        msg.id()
                                    )
                                })?;
                        }
                    }
                }
            }
        }

        Ok(Duration::from_secs(15))
    }

    async fn check_subreddits(&mut self) -> anyhow::Result<Duration> {
        let posts = self
            .posts
            .get_next_batch(roux::util::FetchMethod::Multi, now_utc(), &mut self.client)
            .await?;

        for post in posts {
            let Some((idx, subreddit)) = self
                .subreddits
                .iter_mut()
                .enumerate()
                .find(|v| v.1.name() == post.subreddit())
            else {
                eprintln!(
                    "Received post from subreddit we never asked for? {}",
                    post.permalink()
                );
                continue;
            };

            if !self.subreddits_mask.posts.is_set(idx) {
                // no modules want this subreddit's posts.
                continue;
            }

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

            let mut view = make_view!(self);
            view.run_post(&mut self.modules, subreddit, idx, post, has_seen)
                .await?;
        }
        Ok(Duration::from_secs(15))
    }

    async fn check_sub_comments(&mut self) -> anyhow::Result<Duration> {
        for (idx, subreddit) in self.subreddits.iter_mut().enumerate() {
            if !self.subreddits_mask.comments.is_set(idx) {
                continue;
            }

            let comments = subreddit.data.latest_comments(None, None).await?;

            let mut view = make_view!(self);

            for comment in comments {
                if comment.author() == &self.own_name {
                    continue;
                }

                if self.db.has_seen(comment.name().full())? {
                    continue;
                }

                self.db
                    .set_seen(subreddit.name().as_str(), comment.name().full())?;

                for reg in &mut self.modules {
                    if reg.module.wants().comments() && reg.submask.comments.is_set(idx) {
                        let name = reg.module.name();
                        reg.module
                            .run_comment(&mut view, &comment)
                            .await
                            .with_context(|| {
                                format!("{name}.run_comment({})", comment.name().full())
                            })?;
                    }
                }
            }
        }

        Ok(Duration::from_secs(15))
    }

    async fn check_status_updates(
        &mut self,
        incident: &IncidentWithLive<'_>,
    ) -> anyhow::Result<()> {
        let current_timestamp = incident
            .incident
            .updated_at
            .unwrap_or_else(|| incident.incident.created_at)
            .to_utc();

        if incident
            .live_thread
            .updated_at
            .is_some_and(|last_updated| current_timestamp <= last_updated)
        {
            println!(
                "{} has no updates since {:?}",
                incident.incident.id, incident.live_thread.updated_at
            );
            return Ok(());
        }

        let mut prior = None;
        for update in incident.incident.incident_updates.iter().rev() {
            if incident
                .live_thread
                .updated_at
                .is_some_and(|updated| update.created_at <= updated)
            {
                prior = Some(update.status);
                continue;
            }

            let mut text = match prior {
                Some(prior) if prior == update.status => {
                    format!("# Update\n\n")
                }
                _ => format!("# {:?}\n\n", update.status),
            };

            text.push_str(&update.body);
            prior = Some(update.status);

            self.client
                .update_live_thread(&incident.live_thread.fullname, &text)
                .await?;
        }

        self.db.update_live_incident(
            &incident.incident.id,
            current_timestamp,
            incident.incident.resolved_at.map(|_| current_timestamp),
        )?;

        if incident.incident.resolved_at.is_some() && incident.live_thread.resolved_at.is_none() {
            self.client
                .close_live_thread(&incident.live_thread.fullname)
                .await?;
        }

        Ok(())
    }

    async fn update_status_with(
        &mut self,
        incidents: &[Incident],
        is_summary: bool,
    ) -> anyhow::Result<()> {
        let mut incidents_with_live = Vec::new();

        let components = self.cached_status_components.data(&self.status).await?;

        let mut unseen = self.db.get_unresolved_live_incidents()?;

        for incident in incidents {
            unseen.remove(&incident.id);
            match self.db.get_live_incident(&incident.id)? {
                None => {
                    let any_would_post = self.subreddits.iter().any(|s| {
                        self.subreddits_config
                            .get_status(s.name())
                            .is_some_and(|c| incident.impact >= c.min_impact)
                    });

                    if !any_would_post {
                        // no since starting a live thread for an irrelevant incident.
                        continue;
                    }

                    let mut resources = String::new();

                    resources.push_str("- [Discord status website](https://discordstatus.com)  \n");
                    resources.push_str(
                        "- [Cloudflare status website](https://www.cloudflarestatus.com)  \n",
                    );

                    let _ = write_affected_components_list(&mut resources, incident, &components);

                    let live_thread_id = self
                        .client
                        .create_live_thread(
                            &get_title(incident, 128)?,
                            &format!(
                                "Tracking updates to [a Discord incident/outage]({}).",
                                incident.shortlink
                            ),
                            false,
                            &resources,
                        )
                        .await?;

                    self.db
                        .create_live_incident(&incident.id, &live_thread_id, None)?;

                    if let Some(admin) = self.admin.as_ref() {
                        self.client
                            .invite_live_thread_contributor(&live_thread_id, &admin)
                            .await?;
                    }

                    incidents_with_live.push(IncidentWithLive {
                        incident: BoO::Borrow(incident),
                        live_thread: LiveIncidentPost {
                            incident_id: incident.id.clone(),
                            fullname: live_thread_id,
                            updated_at: None,
                            resolved_at: None,
                        },
                    })
                }
                Some(live_thread) => incidents_with_live.push(IncidentWithLive {
                    incident: BoO::Borrow(incident),
                    live_thread,
                }),
            }
        }

        if is_summary {
            // Webhook only sends updated incident, so any others
            // are expected to be missing. The summary should include
            // all active incidents. Anything missing has
            // likely been resolved, so fetch it to check.
            for id in unseen {
                println!("Did not see incident {id}, fetching");

                let incident = self
                    .status
                    .get_incident(&id)
                    .await
                    .with_context(|| format!("get incident {id}"))?;

                let live_thread = self
                    .db
                    .get_live_incident(&id)?
                    .expect("we just got this ID from the database");

                incidents_with_live.push(IncidentWithLive {
                    incident: BoO::Owned(incident),
                    live_thread,
                });
            }
        }

        for incident in &incidents_with_live {
            self.check_status_updates(incident).await.with_context(|| {
                format!("check live updates for incident {}", incident.incident.id)
            })?;
        }

        for subreddit in &mut self.subreddits {
            if let Some(config) = self.subreddits_config.get_status(subreddit.name()) {
                subreddit
                    .update_status(
                        &self.db,
                        &self.client,
                        &incidents_with_live,
                        is_summary,
                        config,
                    )
                    .await
                    .with_context(|| format!("check status for /r/{}", subreddit.name()))?;
            }
        }

        Ok(())
    }

    async fn check_status(&mut self) -> anyhow::Result<Duration> {
        let summary = self.status.get_summary().await?;
        self.last_status = summary.status.indicator;
        println!(
            "Status is {:?}, with {} incidents",
            summary.status.indicator,
            summary.incidents.len()
        );

        self.update_status_with(&summary.incidents, true).await?;

        Ok(Duration::from_secs(5 * 60))
    }

    async fn check_own_comments(&mut self) -> anyhow::Result<Duration> {
        let comments = self.client.comments(None).await?;

        for comment in comments {
            if !comment.score_hidden() && comment.score() < 0 {
                if matches!(comment.distinguished(), Distinguished::None) {
                    println!(
                        "Removing downvoted {:?} on {:?} by /u/{}",
                        comment.name(),
                        comment.link_title(),
                        comment.link_author()
                    );
                    comment.delete().await?;
                    self.db.set_mistaken(comment.name().full())?;
                    if let Some(webhook) = &mut self.webhook {
                        let message = create_deleted_downvoted_comment(&comment);
                        webhook.send(&message).await?;
                    }
                } else {
                    println!(
                        "NOT removing downvoted {:?} on {:?} by /u/{}",
                        comment.name(),
                        comment.link_title(),
                        comment.link_author()
                    );
                }
            }
        }

        Ok(Duration::from_secs(15))
    }

    async fn handle_webhook_event(
        &mut self,
        event: WebhookEvent,
        ratelimiter: &mut Ratelimiter<Self>,
    ) -> anyhow::Result<()> {
        match event {
            crate::status_tracker::WebhookEvent::IncidentUpdate(incident) => {
                println!("[status-recv] got incident webhook");
                let incident = *incident;
                self.update_status_with(&[incident], false).await?;
            }
            _ => {
                println!("[status-recv] got unknown webhook, scheduling status check to run");
                ratelimiter.run_immediately("check_status");
            }
        };
        Ok(())
    }

    async fn check_module_timers(&mut self) -> anyhow::Result<Duration> {
        let now = Instant::now();

        let mut earliest_next = Duration::MAX;

        for reg in &mut self.modules {
            if !reg.module.wants().timer() {
                continue;
            }

            if reg.next_timer > now {
                continue;
            }

            let mut view = make_view!(self);

            let after = reg
                .module
                .run_timer(&mut view, &mut self.subreddits)
                .await
                .with_context(|| format!("run_timer {}", reg.module.name()))?;

            if after < earliest_next {
                earliest_next = after;
            }
        }

        Ok(earliest_next)
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let (tx, rx) = mpsc::channel();

        if let Some(addr) = &self.status_webhook {
            println!("Starting status webhook at {addr}");
            crate::status_tracker::start_webhook_listener_thread(tx, &addr)
                .with_context(|| format!("status webhook at {addr}"))?;
        }

        let wants = self
            .modules
            .iter()
            .map(|reg| reg.module.wants())
            .reduce(|acc, e| acc | e)
            .unwrap_or_default();

        let mut ratelimiter = Ratelimiter::new();

        if wants.inbox() {
            println!("enabling inbox.");
            ratelimiter.push("check_inbox", |ctx| Box::pin(Self::check_inbox(ctx)));
        }

        if wants.posts() {
            println!("enabling subreddits.");
            ratelimiter.push("check_subreddits", |ctx| {
                Box::pin(Self::check_subreddits(ctx))
            });
        }

        if wants.comments() {
            println!("enabling comments.");
            ratelimiter.push("check_sub_comments", |ctx| {
                Box::pin(Self::check_sub_comments(ctx))
            });
        }

        if wants.timer() {
            println!("enabling timer.");
            ratelimiter.push("check_module_timers", |ctx| {
                Box::pin(Self::check_module_timers(ctx))
            });
        }

        ratelimiter.push("check_own_comments", |ctx| {
            Box::pin(Self::check_own_comments(ctx))
        });
        ratelimiter.push("check_status", |ctx| Box::pin(Self::check_status(ctx)));

        loop {
            while let Ok(event) = rx.try_recv() {
                self.handle_webhook_event(event, &mut ratelimiter).await?;
            }

            let now = Instant::now();
            let next = ratelimiter.run(self, now).await?;

            match rx.recv_timeout(next - now) {
                Ok(event) => self.handle_webhook_event(event, &mut ratelimiter).await?,
                Err(RecvTimeoutError::Disconnected) => bail!("status webhook disconnected"),
                Err(RecvTimeoutError::Timeout) => continue,
            }
        }
    }

    pub async fn send_webhook(
        &mut self,
        message: &mlapibot_webhook::Message,
    ) -> anyhow::Result<()> {
        if let Some(webhook) = &mut self.webhook {
            webhook.send(message).await?;
        }
        Ok(())
    }
}

pub struct ModuleRedditClient<'client> {
    db: &'client MlapiDb,
    analzyers: &'client [Analyzer],
    own_name: &'client String,
    client: &'client RouxClient,
    templates: &'client Tera,
    webhook: &'client mut Option<WebhookClient>,
    imgur: &'client mut Option<ImgurClient>,
    github: &'client mut Option<octocrab::Octocrab>,
    status: &'client StatusClient,
    last_status: &'client StatusIndicator,
    subreddits_config: &'client SubredditsConfig,
    dry_run: bool,
    status_webhook: &'client Option<String>,
    #[allow(unused)]
    admin: &'client Option<String>,
    flair_cache: &'client mut PostFlairCache,
    cached_status_components: &'client StatusComponentCache,
    debug: bool,
}

impl<'client> ModuleRedditClient<'client> {
    pub async fn send_warnings(
        &mut self,
        warnings: Vec<ContextWarning>,
        context: impl Into<String>,
    ) -> anyhow::Result<()> {
        RedditClient::_send_warnings(self.webhook.as_mut(), warnings, context).await
    }

    async fn run_post(
        &mut self,
        modules: &mut [RegisteredModule],
        subreddit: &mut Subreddit,
        idx: usize,
        post: Submission,
        has_seen: bool,
    ) -> anyhow::Result<()> {
        let config = self.subreddits_config.get(subreddit.name());

        let mut action = PostAction::Ignore;

        for reg in modules {
            if reg.module.wants().posts() && reg.submask.posts.is_set(idx) {
                let name = reg.module.name();

                let mod_act = reg
                    .module
                    .run_post(self, subreddit, config, &post, has_seen)
                    .await
                    .with_context(|| format!("{name}.run_post({})", post.name().full()))?
                    .with_module(reg.module.name());

                action = action.join(mod_act);
            }
        }

        match action {
            PostAction::Action(data) if !self.dry_run => {
                data.execute(self.debug, self.webhook.as_mut(), self.db, &post)
                    .await?;
            }
            PostAction::Ignore | PostAction::Action(..) => {
                self.db.set_ignored(post.name().full())?;
            }
        }

        Ok(())
    }
}
