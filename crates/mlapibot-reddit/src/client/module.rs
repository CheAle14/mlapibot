use std::time::{Duration, Instant};

use mlapibot_common::action::{ActionData, ModAct, PostAction};
use mlapibot_database_v2::{
    client::PgClient,
    repos::monitor::{MonitorRepo, MonitorState},
};
use mlapibot_webhook::WebhookClient;
use roux::{
    client::{AuthedClient, RedditClient},
    models::{Distinguish, LatestComment},
};

use crate::{
    RedditMessage, RouxClient, Submission, client::ModuleRedditClient, subreddit::Subreddit,
    webhook::create_detection_message,
};

pub mod comment_cdn_links;
pub mod comment_code;
pub mod comment_complex;
pub mod comment_staff_replies;
pub mod inbox_commands;
pub mod post_ai_slop;
pub mod post_scams;
pub mod post_vague_title;

pub use comment_cdn_links::CdnLinks;
pub use comment_code::CommentCode;
pub use comment_complex::CommentComplex;
pub use comment_staff_replies::CommentStaffReplies;
pub use inbox_commands::InboxCommands;
pub use post_ai_slop::PostAiSlop;
pub use post_scams::PostScams;
pub use post_vague_title::PostVagueTitle;

#[expect(unused_variables)]
#[async_trait::async_trait(?Send)]
pub trait Module {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &'static str;
    fn wants(&self) -> ModuleWants;

    fn as_staff_replies(&mut self) -> Option<&mut CommentStaffReplies> {
        None
    }

    fn mask_subreddits(&self, subreddits: &[Subreddit]) -> SplitSubMask {
        SplitSubMask::new()
    }

    async fn run_post<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddit: &mut Subreddit,
        post: &Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        Ok(PostAction::Ignore)
    }

    async fn run_comment<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddits: &mut Subreddit,
        comment: &LatestComment<AuthedClient>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_inbox<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddits: &mut [Subreddit],
        msg: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<InboxAction>> {
        Ok(None)
    }

    async fn run_timer<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddits: &mut [Subreddit],
    ) -> anyhow::Result<Duration> {
        Ok(Duration::MAX)
    }
}

pub struct RegisteredModule {
    pub submask: SplitSubMask,
    pub module: Box<dyn Module>,
    pub next_timer: Instant,
}

impl super::RedditClient {
    pub(super) fn build_modules(subreddits: &[Subreddit]) -> (SplitSubMask, Vec<RegisteredModule>) {
        macro_rules! modules {
            ($($name:ident),* $(,)?) => {{
                let mut sub_mask = SplitSubMask::new();
                let mut modules = Vec::new();
                let now = Instant::now();

                $(
                    let mdl = <$name as Module>::new();
                    let mask = Module::mask_subreddits(&mdl, subreddits);

                    sub_mask |= mask;

                    modules.push(RegisteredModule {
                        submask: mask,
                        module: Box::new(mdl) as Box<dyn Module>,
                        next_timer: now,
                    });
                )*

                (sub_mask, modules)
            }};
        }

        modules!(
            PostScams,
            CommentCode,
            InboxCommands,
            PostVagueTitle,
            CdnLinks,
            CommentStaffReplies,
            CommentComplex,
            PostAiSlop,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleWants(u8);

impl std::fmt::Debug for ModuleWants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ModuleWants(")?;

        let mut any = false;

        macro_rules! check {
            ($name:ident) => {
                if self.has(Self::$name) {
                    if any {
                        write!(f, "| ")?;
                    }
                    any = true;
                    write!(f, stringify!($name))?;
                }
            };
        }

        check!(POSTS);
        check!(COMMENTS);
        check!(INBOX);
        check!(TIMER);

        let _ = any;

        write!(f, ")")
    }
}

impl ModuleWants {
    pub const POSTS: ModuleWants = ModuleWants(1 << 0);
    pub const COMMENTS: ModuleWants = ModuleWants(1 << 1);
    pub const INBOX: ModuleWants = ModuleWants(1 << 2);
    pub const TIMER: ModuleWants = ModuleWants(1 << 3);
    pub const MODQUEUE: ModuleWants = ModuleWants(1 << 4);

    pub fn has(&self, wants: ModuleWants) -> bool {
        (*self & wants).0 != 0
    }

    pub fn posts(&self) -> bool {
        self.has(ModuleWants::POSTS)
    }

    pub fn comments(&self) -> bool {
        self.has(ModuleWants::COMMENTS)
    }

    pub fn inbox(&self) -> bool {
        self.has(ModuleWants::INBOX)
    }

    pub fn timer(&self) -> bool {
        self.has(ModuleWants::TIMER)
    }

    pub fn modqueue(&self) -> bool {
        self.has(ModuleWants::MODQUEUE)
    }
}

impl std::ops::BitOr for ModuleWants {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for ModuleWants {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SubMask(u32);

impl std::fmt::Debug for SubMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubMask({:b})", self.0)
    }
}

impl SubMask {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn has_any(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, idx: usize) {
        self.0 |= 1 << idx;
    }

    pub fn is_set(&self, idx: usize) -> bool {
        (self.0 & (1u32 << idx)) != 0
    }
}

impl std::ops::BitOrAssign for SubMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitSubMask {
    pub posts: SubMask,
    pub comments: SubMask,
    pub modqueue: SubMask,
}

impl SplitSubMask {
    pub fn new() -> Self {
        Self {
            posts: SubMask::new(),
            comments: SubMask::new(),
            modqueue: SubMask::new(),
        }
    }
}

impl std::ops::BitOrAssign for SplitSubMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.posts |= rhs.posts;
        self.comments |= rhs.comments;
        self.modqueue |= rhs.modqueue;
    }
}

pub enum InboxAction {
    RedoSub(Submission),
    RedoMsg(Submission, LatestComment<RouxClient>),
}

macro_rules! impl_mask_subreddits {
    (
        $flag:ident => $($wants:ident),*
    ) => {
        fn mask_subreddits(
            &self,
            subreddits: &[crate::subreddit::Subreddit],
        ) -> super::SplitSubMask {
            let mut sum = super::SplitSubMask::new();
            for (idx, sub) in subreddits.iter().enumerate() {
                if  sub.db.$flag.enabled
                {
                    $(
                        sum.$wants.set(idx);
                    )*
                }
            }

            sum
        }
    };
}

pub(self) use impl_mask_subreddits;

pub struct InboxMsg<'a> {
    inner: &'a RedditMessage,
    pub subject: &'a str,
    pub author: &'a str,
    pub body: &'a str,
}

impl<'a> InboxMsg<'a> {
    pub fn new(inner: &'a RedditMessage) -> (Self, bool) {
        let subject = inner.subject();
        let (subject, body) = if subject == "[direct chat room]" {
            match inner.body().split_once('\n') {
                Some(pair) => pair,
                // If body has only one line, that is new subject,
                // and the body is empty
                None => (inner.body(), ""),
            }
        } else {
            (subject, inner.body())
        };

        let author = inner.author().unwrap_or_default();

        let (subject, dev_only) = match subject.strip_prefix("[dev-only]") {
            Some(rem) => (rem.trim_start(), true),
            None => (subject, false),
        };

        (
            Self {
                inner,
                author,
                subject,
                body,
            },
            dev_only,
        )
    }

    async fn reply(
        &self,
        content: &str,
    ) -> Result<roux::models::Message<roux::client::AuthedClient>, roux::util::RouxError> {
        self.inner.reply(content).await
    }
}

pub async fn execute(
    action: ActionData,
    is_debug: bool,
    webhook: Option<&mut WebhookClient>,
    db: &PgClient,
    post: &Submission,
    client: &RouxClient,
) -> anyhow::Result<()> {
    let reply_fullname = if let Some(reply) = action.reply {
        let comment = post.comment(&reply.text).await?;
        if reply.distinguish {
            comment.distinguish(Distinguish::Moderator, true).await?;
        }

        Some(comment.name().full().to_string())
    } else {
        None
    };

    let (reported, removed, webhook_text) = match action.moderate {
        ModAct::None => (false, false, None),
        ModAct::Report { reason } => {
            post.report(&reason).await?;

            (true, false, Some(reason))
        }
        ModAct::Remove => {
            post.remove(false).await?;
            (false, true, None)
        }
        ModAct::Filter => {
            post.remove(false).await?;

            let mut modmail = match action.analyser.as_ref() {
                Some(c) => format!("Filtered post for manual review, related to {c}"),
                None => format!("Filtered post for manual review"),
            };

            modmail.push_str("\n\nTitle:  \n>");
            modmail.push_str(post.title());

            modmail.push_str("\n\nLink: ");
            modmail.push_str(post.permalink());

            let sub = client.subreddit(&post.subreddit());

            sub.compose_message(&format!("Filtered post by /u/{}", post.author()), &modmail)
                .await?;

            (true, true, None)
        }
    };

    db.update_item_monitor_state(
        post.name().full(),
        MonitorState::Acted {
            analyzer: action
                .analyser
                .as_ref()
                .map(|c| c.clone())
                .unwrap_or_default(),
            reply_fullname,
            reported,
            removed,
            mistaken: false,
        },
    )
    .await?;

    if let Some(webhook) = webhook {
        let msg = create_detection_message(
            post,
            &action.module,
            action.analyser.as_ref().map(|c| c.as_str()),
            webhook_text.as_ref().map(|c| c.as_str()),
            is_debug,
        );
        webhook.send(&msg).await?;
    }

    Ok(())
}
