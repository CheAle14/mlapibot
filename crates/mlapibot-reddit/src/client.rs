use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Context;
use chrono::Utc;
use mlapibot_api::{ApiEvent, GotAnalysis, GotRedditPost, OcrImageData, RefreshStaffResponse};
use mlapibot_common::{
    Cached, LowercaseString, Words,
    action::PostAction,
    config::{ApiSettings, GlobalSettings},
};
use mlapibot_database_v2::{
    client::{PgClient, PgClientBuilder, PgNotification},
    repos::{
        incidents::{IncidentRepo, StatusIncident},
        monitor::MonitorRepo,
        subreddits::{ScheduledPostId, ScheduledPostSticky, SubredditsRepo},
    },
};
use octocrab::OctocrabBuilder;
use roux::{
    api::{Distinguished, ThingFullname},
    builders::submission::SubmissionSubmitBuilder,
    client::{AuthedClient, OAuthClient, RedditClient as RouxRedditClient},
    models::{
        LatestComment,
        modqueue::{Modqueue, QueueThing},
    },
    util::{SubmissionStream, now_utc},
};
use statuspage::{StatusClient, component::Component, incident::Incident, status::StatusIndicator};

use mlapibot_analysis::{ContextWarning, Url};
use mlapibot_imgur::ImgurClient;
use mlapibot_webhook::{WebhookClient, create_multiple_error_message};
use tokio::sync::mpsc;

use super::{RouxClient, Submission};

use crate::{
    Comment,
    client::module::{InboxAction, InboxMsg, RegisteredModule, SplitSubMask},
    exts::{ModerationExt, SubmissionExt},
    ratelimiter::Ratelimiter,
    status_tracker::{IncidentWithLive, get_title, write_affected_components_list},
    subreddit::{DbSubreddit, Subreddit},
    utils::BoO,
    webhook::{create_deleted_downvoted_comment, create_inbox_message},
};

pub mod module;

pub struct RedditClient {
    // data_dir: PathBuf,
    db: PgClient,
    own_name: String,
    pub client: RouxClient,
    subreddits: Vec<Subreddit>,
    webhook: Option<WebhookClient>,
    imgur: Option<ImgurClient>,
    api: Option<ApiSettings>,
    github: Option<octocrab::Octocrab>,
    status: StatusClient,
    last_status: StatusIndicator,
    dry_run: bool,
    #[allow(unused)]
    admin: Option<String>,
    cached_status_components: StatusComponentCache,
    debug: bool,
    dbrx: mpsc::Receiver<PgNotification>,

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
            own_name: &$self.own_name,
            client: &$self.client,
            webhook: &mut $self.webhook,
            imgur: &mut $self.imgur,
            github: &mut $self.github,
            status: &$self.status,
            last_status: &$self.last_status,
            dry_run: $self.dry_run,
            admin: &$self.admin,
            cached_status_components: &$self.cached_status_components,
            debug: $self.debug,
        }
    };
}

impl RedditClient {
    async fn fetch_and_sync_subreddits(
        db: &mut PgClient,
        reddit: &RouxClient,
    ) -> anyhow::Result<Vec<DbSubreddit>> {
        let mut in_db = db
            .fetch_all_subreddits()
            .await
            .context("fetch subreddits")?;

        let from_api = reddit
            .get_my_subreddits(roux::client::SubRelation::Moderator)
            .await?;

        // for sub in &mut in_db {
        //     if sub.enabled && !from_api.data.children.iter().any(|x| x.data.id == sub.id) {
        //         println!("Disabling subreddit /r/{} {}", sub.name, sub.id);
        //         db.set_subreddit_enabled(&sub.id, false).await?;
        //         sub.enabled = false;
        //     }
        // }

        for api in from_api.data.children {
            if let Some(s) = in_db.iter_mut().find(|s| s.id == api.data.id) {
                if !s.enabled {
                    db.set_subreddit_enabled(&s.id, true).await?;
                    s.enabled = true;
                }
                continue;
            }

            let new = DbSubreddit::new(&api.data.id, &api.data.display_name);

            println!(
                "Registering new subreddit /r/{} @ {}",
                api.data.display_name, api.data.id
            );

            db.create_subreddit(&new)
                .await
                .with_context(|| format!("creating {} {}", api.data.id, api.data.display_name))?;

            in_db.push(new);
        }

        in_db.retain(|a| a.enabled);

        let now = Utc::now();
        for subreddit in &mut in_db {
            let diff = now.signed_duration_since(subreddit.last_sync);

            if diff.num_days() >= 7 {
                let mods = match reddit.subreddit(&subreddit.name).moderators().await {
                    Ok(m) => m,
                    Err(err) => {
                        eprintln!("Failed to get mods for /r/{}: {err}", subreddit.name);
                        continue;
                    }
                };

                let names: Vec<_> = mods
                    .data
                    .children
                    .iter()
                    .map(|data| data.name.as_str())
                    .collect();

                db.set_subreddit_moderators(&subreddit.id, &names)
                    .await
                    .with_context(|| format!("set moderators {}", subreddit.id))?;
            }
        }

        Ok(in_db)
    }

    async fn convert_db_subreddits(
        db: &mut PgClient,
        client: &RouxClient,
        db_subreddits: Vec<DbSubreddit>,
    ) -> anyhow::Result<Vec<Subreddit>> {
        let mut subreddits = Vec::with_capacity(db_subreddits.len());

        for db_sub in db_subreddits {
            let moderators = db
                .get_subreddit_moderators(&db_sub.id)
                .await
                .with_context(|| format!("get sub mods {}", db_sub.id))?;
            let moderators: HashSet<String> = moderators.into_iter().collect();

            let templates = db.get_subreddit_templates(&db_sub.id).await?;
            let scams = db.get_subreddit_scams(&db_sub.id).await?;

            let name = LowercaseString::new(&db_sub.name);

            let sub = Subreddit::new(
                client.subreddit(&db_sub.name),
                db_sub,
                templates,
                scams,
                moderators,
                name,
            );

            match sub.await {
                Ok(sub) => subreddits.push(sub),
                Err(err) => {
                    eprintln!("failed to create subreddit: {err}")
                }
            }
        }

        Ok(subreddits)
    }

    fn make_submission_stream(
        subreddits: &[Subreddit],
        subreddits_mask: SplitSubMask,
    ) -> SubmissionStream<Submission> {
        SubmissionStream::new(
            25,
            subreddits.iter().enumerate().filter_map(|(idx, sub)| {
                if subreddits_mask.posts.is_set(idx) {
                    Some(sub.name().as_str())
                } else {
                    None
                }
            }),
        )
    }

    pub async fn new(
        _data_dir: PathBuf,
        dry_run: bool,
        settings: GlobalSettings,
        debug: bool,
    ) -> anyhow::Result<Self> {
        let reddit = settings
            .reddit
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("reddit config missing from settings"))?;

        let (dbtx, dbrx) = tokio::sync::mpsc::channel(32);

        let mut db = PgClientBuilder::new(&settings.database_uri)
            .listen(dbtx)
            .connect()
            .await
            .context("initialize db")?;

        let config =
            roux::Config::new(&reddit.user_agent, &reddit.client_id, &reddit.client_secret)
                .username(&reddit.username)
                .password(&reddit.password)
                .timeout(std::time::Duration::from_mins(1));

        println!("Logging in now...");
        let client = OAuthClient::new(config)?.login().await?;
        println!("Logged in!");

        let me = client.me().await?;

        println!("me: {me:#?}");

        tokio::time::sleep(Duration::from_secs(10)).await;

        println!("Fetching and syncing subreddits");

        let db_subreddits = Self::fetch_and_sync_subreddits(&mut db, &client).await?;

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
        let subreddits = Self::convert_db_subreddits(&mut db, &client, db_subreddits).await?;

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
            "Logged in as /u/{username}; monitoring {subs} subreddits in {mode}",
            username = reddit.username,
            subs = subreddits.len(),
            mode = if debug { "debug mode" } else { "release mode" }
        );

        if dry_run {
            println!("Running in dry-run mode.");
        }

        let (subreddits_mask, modules) = Self::build_modules(&subreddits);
        let posts = Self::make_submission_stream(&subreddits, subreddits_mask);

        Ok(Self {
            db,
            dbrx,
            own_name: reddit.username.clone(),
            client,
            subreddits,
            // data_dir: scratch_dir,
            webhook,
            imgur,
            api: settings.api,
            admin: reddit.admin.clone(),
            github,
            status,
            dry_run: dry_run,
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
                                .run_comment(&mut view, &mut self.subreddits[idx], &msg)
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
                .has_seen_item(post.name().full())
                .await
                .context("lookup seen")?;

            if post.has_unknown_media() {
                continue;
            } else if !has_seen {
                self.db
                    .mark_item_seen(post.name().full(), subreddit.name().as_str())
                    .await
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

            let mut after_fullname: Option<ThingFullname> = None;
            let mut limit = 25;
            let mut seen_any = false;
            let mut can_continue = true;

            while can_continue && !seen_any {
                let mut comments = subreddit
                    .reddit
                    .latest_comments(
                        None,
                        Some(std::cmp::min(limit, 100)),
                        after_fullname.as_ref(),
                    )
                    .await?;

                can_continue = comments.after.is_some();
                after_fullname = comments.after.take();

                let mut view = make_view!(self);

                for comment in comments {
                    if comment.author() == &self.own_name {
                        continue;
                    }

                    if self.db.has_seen_item(comment.name().full()).await? {
                        seen_any = true;
                        continue;
                    }

                    self.db
                        .mark_item_seen(comment.name().full(), subreddit.name().as_str())
                        .await?;

                    view.run_comment(&mut self.modules, subreddit, idx, comment)
                        .await?;
                }

                limit = limit * 2;
            }
        }

        Ok(Duration::from_secs(15))
    }

    async fn check_sub_modqueue(&mut self) -> anyhow::Result<Duration> {
        for (idx, subreddit) in self.subreddits.iter_mut().enumerate() {
            if !self.subreddits_mask.modqueue.is_set(idx) {
                continue;
            }

            let queue = subreddit.reddit.modqueue(None).await?;

            let mut view = make_view!(self);
            for thing in queue {
                if thing.author() == &self.own_name {
                    continue;
                }

                match &thing {
                    QueueThing::Submission(d) => {
                        if d.has_unknown_media() || d.has_any_mod_action_by_human() {
                            continue;
                        }
                    }
                    QueueThing::Comment(d) => {
                        if d.has_any_mod_action_by_human() {
                            continue;
                        }
                    }
                }

                let has_seen = self.db.has_seen_item(thing.name().full()).await?;

                if !has_seen {
                    self.db
                        .mark_item_seen(thing.name().full(), subreddit.name().as_str())
                        .await?;

                    println!("Saw mod {:?} by /u/{}", thing.name().full(), thing.author(),);
                }

                match thing {
                    QueueThing::Submission(post) => {
                        view.run_post(&mut self.modules, subreddit, idx, post, has_seen)
                            .await?;
                    }
                    QueueThing::Comment(comment) => {
                        if !has_seen {
                            view.run_comment(&mut self.modules, subreddit, idx, comment)
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(Duration::from_mins(1))
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
                .update_live_thread(&incident.live_thread.live_fullname, &text)
                .await?;
        }

        self.db
            .update_status_incident(
                &incident.incident.id,
                current_timestamp,
                incident.incident.resolved_at.map(|_| current_timestamp),
            )
            .await?;

        if incident.incident.resolved_at.is_some() && incident.live_thread.resolved_at.is_none() {
            self.client
                .close_live_thread(&incident.live_thread.live_fullname)
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

        let mut unseen = self.db.get_unresolved_status_incidents().await?;

        for incident in incidents {
            unseen.remove(&incident.id);
            match self.db.get_status_incident_by_id(&incident.id).await? {
                None => {
                    let any_would_post = self.subreddits.iter().any(|s| {
                        s.db.mod_status.enabled && incident.impact >= s.db.mod_status.min_impact
                    });

                    if !any_would_post {
                        // no point starting a live thread for an irrelevant incident.
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
                        .create_status_incident(&incident.id, &live_thread_id, None)
                        .await?;

                    if let Some(admin) = self.admin.as_ref() {
                        self.client
                            .invite_live_thread_contributor(&live_thread_id, &admin)
                            .await?;
                    }

                    incidents_with_live.push(IncidentWithLive {
                        incident: BoO::Borrow(incident),
                        live_thread: StatusIncident {
                            incident_id: incident.id.clone(),
                            live_fullname: live_thread_id,
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
                    .get_status_incident_by_id(&id)
                    .await?
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
            if subreddit.db.mod_status.enabled {
                subreddit
                    .update_status(&self.db, &self.client, &incidents_with_live, is_summary)
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
                    self.db
                        .set_item_mistaken(comment.name().full(), true)
                        .await?;
                    if let Some(webhook) = &mut self.webhook {
                        let message = create_deleted_downvoted_comment(&comment);
                        webhook.send(&message).await?;
                    }
                }
            }
        }

        Ok(Duration::from_secs(300))
    }

    async fn handle_api_event(
        &mut self,
        event: ApiEvent,
        ratelimiter: &mut Ratelimiter<Self>,
    ) -> anyhow::Result<()> {
        match event {
            ApiEvent::WebhookRecv { incident } => {
                if let Some(incident) = incident {
                    println!("[status-recv] got incident webhook");
                    let incident = *incident;
                    self.update_status_with(&[incident], false).await?;
                } else {
                    println!("[status-recv] got unknown webhook, scheduling status check to run");
                    ratelimiter.run_immediately("check_status");
                }
            }
            ApiEvent::GetRedditPost { link, reply } => {
                let Ok(post) = self.client.get_submission_by_link(&link).await else {
                    // drops reply, returns an error.
                    return Ok(());
                };

                let links = post
                    .get_misc_links()
                    .into_iter()
                    .map(|v| v.as_str().to_owned())
                    .collect();

                let post = GotRedditPost {
                    id: post.id().to_owned(),
                    subreddit_id: post.subreddit_id().id().to_owned(),
                    title: post.title().to_owned(),
                    author: post.author().to_owned(),
                    links,
                    body: Some(post.selftext().clone()),
                };

                let _ = reply.send(post);
            }
            ApiEvent::RefreshStaffReply {
                subreddit_name,
                post_id,
                reply,
            } => {
                let Some(subreddit) = self
                    .subreddits
                    .iter_mut()
                    .find(|s| s.name() == subreddit_name.as_str())
                else {
                    return Ok(());
                };

                for module in &mut self.modules {
                    let Some(module) = module.module.as_staff_replies() else {
                        continue;
                    };

                    let mut client = make_view!(self);

                    let counts = module
                        .update_or_make_staff_reply_comment(
                            &mut client,
                            &subreddit_name,
                            &post_id,
                            &subreddit.db.mod_staff_reply,
                            true,
                        )
                        .await
                        .context("api redo")?;

                    let _ = reply.send(RefreshStaffResponse {
                        total_comments: counts.total,
                        total_staff_comments: counts.staff,
                        new_staff_comments: counts.new_staff,
                    });

                    break;
                }
            }
            ApiEvent::AnalyzeInfo {
                subreddit_id,
                title,
                links,
                body,

                reply,
            } => {
                let Some(subreddit) = self.subreddits.iter_mut().find(|s| s.db.id == subreddit_id)
                else {
                    return Ok(());
                };

                let links: Vec<_> = links
                    .into_iter()
                    .filter_map(|s| Url::parse(&s).ok())
                    .collect();

                let title = Words::new(title);
                let body = body.map(Words::new);

                let (action, ctx, det) = crate::client::module::post_scams::analyze_post(
                    &mut (),
                    subreddit,
                    "",
                    title.full_text(),
                    links.into_iter(),
                    body.as_ref().map(|v| v.full_text()).unwrap_or_default(),
                    true,
                )
                .await?;

                let mut ocr: Vec<OcrImageData> = Vec::new();

                // Unconditionally add OCR, to always see what the bot sees.
                for (idx, v) in ctx.images.iter().enumerate() {
                    let triggers = det
                        .as_ref()
                        .and_then(|det| det.images.get(&idx))
                        .map(|v| v.words.keys().copied().collect::<Vec<_>>())
                        .unwrap_or_default();

                    ocr.push(OcrImageData {
                        text: v.full_text(),
                        name: v.name.clone().unwrap_or_else(|| format!("<unnamed image>")),
                        triggers,
                    });
                }

                if let Some(det) = &det {
                    // Only add title/body to mark any detections.
                    // Presumably they already know what text is in there.
                    if let Some(det_title) = &det.title {
                        ocr.push(OcrImageData {
                            name: String::from("<title>"),
                            text: ctx
                                .title
                                .map(|v| v.full_text().to_owned())
                                .unwrap_or_default(),
                            triggers: det_title.words.keys().copied().collect(),
                        });
                    }

                    if let Some(det_body) = &det.body {
                        ocr.push(OcrImageData {
                            name: String::from("<body>"),
                            text: ctx
                                .body
                                .map(|v| v.full_text().to_owned())
                                .unwrap_or_default(),
                            triggers: det_body.words.keys().copied().collect(),
                        });
                    }
                }

                let response = GotAnalysis { action, ocr };

                let _ = reply.send(response);
            }
            ApiEvent::PublishPost { id, reply } => {
                let Some(post) = self.db.get_scheduled_post(ScheduledPostId::new(id)).await? else {
                    let _ = reply.send(None);
                    return Ok(());
                };

                if let Some(existing) = &post.reddit_id {
                    // We are publishing an update.

                    let fullname = ThingFullname::from_submission_id(existing);

                    self.client.edit(&post.content, &fullname).await?;
                    self.db.publish_scheduled_post(post.id, &existing).await?;

                    let _ = reply.send(Some(existing.clone()));

                    return Ok(());
                }

                let Some(subreddit) = self
                    .subreddits
                    .iter()
                    .find(|s| s.db.id == post.subreddit_id)
                else {
                    let _ = reply.send(None);
                    return Ok(());
                };

                let client = self.client.subreddit(subreddit.name().as_str());
                let builder = SubmissionSubmitBuilder::text(&post.title, &post.content)
                    .with_send_replies(false);

                let builder = if let Some(id) = post.flair_id {
                    builder.with_flair_id(id)
                } else {
                    builder
                };

                let builder = if let Some(txt) = post.flair_text {
                    builder.with_flair_text(txt)
                } else {
                    builder
                };

                let submission = client.submit(&builder).await?;
                let _ = reply.send(Some(submission.id().clone()));

                self.db
                    .publish_scheduled_post(post.id, &submission.id())
                    .await?;

                if submission.moderation().is_some() {
                    if post.distinguish {
                        submission
                            .distinguish(roux::models::Distinguish::Moderator)
                            .await?;
                    }

                    if post.lock {
                        submission.lock().await?;
                    }

                    match post.sticky {
                        ScheduledPostSticky::None => (),
                        ScheduledPostSticky::Top => {
                            submission
                                .sticky(true, roux::models::SubmissionStickySlot::Top)
                                .await?
                        }
                        ScheduledPostSticky::Bottom => {
                            submission
                                .sticky(true, roux::models::SubmissionStickySlot::Bottom)
                                .await?
                        }
                    }
                }
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

    fn reconfigure_ratelimit(modules: &[RegisteredModule], ratelimiter: &mut Ratelimiter<Self>) {
        let mut overall_sub_mask = SplitSubMask::new();

        for reg in modules {
            overall_sub_mask |= reg.submask;
            println!(
                "{} wants {:?}\n => {:?}",
                reg.module.name(),
                reg.module.wants(),
                reg.submask
            );
        }

        let wants = modules
            .iter()
            .map(|reg| reg.module.wants())
            .reduce(|acc, e| acc | e)
            .unwrap_or_default();

        macro_rules! set {
            ($wantsFn:ident, $callback:ident) => {
                #[allow(non_upper_case_globals)]
                const $wantsFn: &'static str = stringify!($callback);

                if wants.$wantsFn() {
                    if !ratelimiter.has($wantsFn) {
                        println!("enabling {}.", $wantsFn);
                        ratelimiter.push($wantsFn, |ctx| Box::pin(Self::$callback(ctx)));
                    }
                } else if ratelimiter.remove($wantsFn) {
                    println!("disabled {}.", $wantsFn);
                }
            };
            ($wantsFn:ident + $mask:ident, $callback:ident) => {
                #[allow(non_upper_case_globals)]
                const $wantsFn: &'static str = stringify!($callback);

                if wants.$wantsFn() && overall_sub_mask.$mask.has_any() {
                    if !ratelimiter.has($wantsFn) {
                        println!("enabling {}.", $wantsFn);
                        ratelimiter.push($wantsFn, |ctx| Box::pin(Self::$callback(ctx)));
                    }
                } else if ratelimiter.remove($wantsFn) {
                    println!("disabled {}.", $wantsFn);
                }
            };
        }

        set!(inbox, check_inbox);
        set!(posts + posts, check_subreddits);
        set!(comments + comments, check_sub_comments);
        set!(modqueue + modqueue, check_sub_modqueue);
        set!(timer, check_module_timers);

        ratelimiter.push("check_own_comments", |ctx| {
            Box::pin(Self::check_own_comments(ctx))
        });

        ratelimiter.push("check_status", |ctx| Box::pin(Self::check_status(ctx)));
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel(32);

        if let Some(api) = &self.api {
            println!("Starting status webhook at {}", api.bind_address);
            mlapibot_api::start_web_connection(tx, &api)
                .with_context(|| format!("start api @ {}", api.bind_address))?;
        }

        let mut ratelimiter = Ratelimiter::new();
        Self::reconfigure_ratelimit(&self.modules, &mut ratelimiter);

        loop {
            while let Ok(event) = rx.try_recv() {
                self.handle_api_event(event, &mut ratelimiter).await?;
            }

            let now = Instant::now();
            let next = ratelimiter.run(self, now).await?;

            println!("Next task in {:?}", next - now);

            let _ = tokio::select! {
                Some(webhook) = rx.recv() => {
                    self.handle_api_event(webhook, &mut ratelimiter).await?;
                }
                Some(dbmsg) = self.dbrx.recv() => {
                    println!("db msg: {dbmsg:?}");
                    self.handle_db_notification(&mut ratelimiter, dbmsg).await?;
                }
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(next)) => {
                    continue;
                }
            };
        }
    }

    async fn handle_db_notification(
        &mut self,
        ratelimiter: &mut Ratelimiter<Self>,
        // For now we just refresh everything.
        _msg: PgNotification,
    ) -> anyhow::Result<()> {
        let db_subreddits = Self::fetch_and_sync_subreddits(&mut self.db, &self.client).await?;
        let subreddits =
            Self::convert_db_subreddits(&mut self.db, &self.client, db_subreddits).await?;
        let (subreddits_mask, modules) = Self::build_modules(&subreddits);
        let posts = Self::make_submission_stream(&subreddits, subreddits_mask);

        self.subreddits = subreddits;
        self.subreddits_mask = subreddits_mask;
        self.modules = modules;
        self.posts = posts;

        Self::reconfigure_ratelimit(&self.modules, ratelimiter);
        Ok(())
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

#[allow(unused)] // future modules may use thems
pub struct ModuleRedditClient<'client> {
    db: &'client PgClient,
    own_name: &'client String,
    client: &'client RouxClient,
    webhook: &'client mut Option<WebhookClient>,
    imgur: &'client mut Option<ImgurClient>,
    github: &'client mut Option<octocrab::Octocrab>,
    status: &'client StatusClient,
    last_status: &'client StatusIndicator,
    dry_run: bool,
    #[allow(unused)]
    admin: &'client Option<String>,
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
        let mut action = PostAction::Ignore;

        for reg in modules {
            if reg.module.wants().posts() && reg.submask.posts.is_set(idx) {
                let name = reg.module.name();

                let mod_act = reg
                    .module
                    .run_post(self, subreddit, &post, has_seen)
                    .await
                    .with_context(|| format!("{name}.run_post({})", post.name().full()))?
                    .with_module(reg.module.name());

                action = action.join(mod_act);
            }
        }

        match action {
            PostAction::Action(data) if !self.dry_run => {
                crate::client::module::execute(
                    data,
                    self.debug,
                    self.webhook.as_mut(),
                    self.db,
                    &post,
                    self.client,
                )
                .await?;
            }
            PostAction::Ignore | PostAction::Action(..) => {
                self.db
                    .update_item_monitor_state(
                        post.name().full(),
                        mlapibot_database_v2::repos::monitor::MonitorState::Ignored,
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn run_comment(
        &mut self,
        modules: &mut [RegisteredModule],
        subreddit: &mut Subreddit,
        idx: usize,
        comment: LatestComment<AuthedClient>,
    ) -> anyhow::Result<()> {
        for reg in modules {
            if reg.module.wants().comments() && reg.submask.comments.is_set(idx) {
                let name = reg.module.name();
                reg.module
                    .run_comment(self, subreddit, &comment)
                    .await
                    .with_context(|| format!("{name}.run_comment({})", comment.name().full()))?;
            }
        }

        Ok(())
    }
}
