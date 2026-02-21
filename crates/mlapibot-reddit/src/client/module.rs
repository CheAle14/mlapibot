use std::time::{Duration, Instant};

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
    RedditMessage, RouxClient, Submission,
    client::ModuleRedditClient,
    config::{SubredditConfig, SubredditsConfig},
    subreddit::Subreddit,
    webhook::create_detection_message,
};

pub mod comment_cdn_links;
pub mod comment_code;
pub mod comment_complex;
pub mod comment_staff_replies;
pub mod inbox_commands;
pub mod post_ai_slop;
pub mod post_flairs;
pub mod post_scams;
pub mod post_vague_title;

pub use comment_cdn_links::CdnLinks;
pub use comment_code::CommentCode;
pub use comment_complex::CommentComplex;
pub use comment_staff_replies::CommentStaffReplies;
pub use inbox_commands::InboxCommands;
pub use post_ai_slop::PostAiSlop;
pub use post_flairs::PostFlairs;
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

    fn mask_subreddits(&self, config: &SubredditsConfig, subreddits: &[Subreddit]) -> SplitSubMask {
        SplitSubMask::new()
    }

    async fn run_post<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&SubredditConfig>,
        post: &Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        Ok(PostAction::Ignore)
    }

    async fn run_comment<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
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

impl<'a> super::RedditClient<'a> {
    pub(super) fn build_modules(
        config: &SubredditsConfig,
        subreddits: &[Subreddit],
    ) -> (SplitSubMask, Vec<RegisteredModule>) {
        macro_rules! modules {
            ($($name:ident),* $(,)?) => {{
                let mut sub_mask = SplitSubMask::new();
                let mut modules = Vec::new();
                let now = Instant::now();

                $(
                    let mdl = <$name as Module>::new();
                    let mask = Module::mask_subreddits(&mdl, config, subreddits);

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
            PostFlairs,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleWants(u8);

impl ModuleWants {
    pub const POSTS: ModuleWants = ModuleWants(0b0001);
    pub const COMMENTS: ModuleWants = ModuleWants(0b0010);
    pub const INBOX: ModuleWants = ModuleWants(0b0100);
    pub const TIMER: ModuleWants = ModuleWants(0b1000);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubMask(u32);

impl SubMask {
    fn all(len: usize) -> Self {
        Self(2u32.pow(len as u32) - 1)
    }

    pub fn new() -> Self {
        Self(0)
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
}

impl SplitSubMask {
    fn all(len: usize) -> Self {
        Self {
            posts: SubMask::all(len),
            comments: SubMask::all(len),
        }
    }

    pub fn new() -> Self {
        Self {
            posts: SubMask::new(),
            comments: SubMask::new(),
        }
    }
}

impl std::ops::BitOrAssign for SplitSubMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.posts |= rhs.posts;
        self.comments |= rhs.comments;
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
            config: &crate::config::SubredditsConfig,
            subreddits: &[crate::subreddit::Subreddit],
        ) -> super::SplitSubMask {
            let mut sum = super::SplitSubMask::new();
            for (idx, sub) in subreddits.iter().enumerate() {
                if config
                    .get(sub.name())
                    .map(|c| super::AsBool::as_bool(&c.$flag))
                    .unwrap_or_default()
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

trait AsBool {
    fn as_bool(&self) -> bool;
}

impl AsBool for bool {
    fn as_bool(&self) -> bool {
        *self
    }
}

impl<T> AsBool for Option<T> {
    fn as_bool(&self) -> bool {
        self.is_some()
    }
}

impl<T> AsBool for Vec<T> {
    fn as_bool(&self) -> bool {
        self.len() > 0
    }
}

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

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PostAction {
    #[default]
    Ignore,
    Action(ActionData),
}

impl PostAction {
    pub fn with_module(self, name: &str) -> PostAction {
        match self {
            PostAction::Ignore => PostAction::Ignore,
            PostAction::Action(mut data) => {
                data.set_module(name);
                PostAction::Action(data)
            }
        }
    }

    pub fn join(self, other: PostAction) -> PostAction {
        match (self, other) {
            (PostAction::Ignore, other) => other,
            (this, PostAction::Ignore) => this,

            (PostAction::Action(this_data), PostAction::Action(other_data)) => {
                let data = match (this_data.moderate, other_data.moderate) {
                    (ModAct::None, ModAct::None)
                    | (ModAct::Report, ModAct::Report)
                    | (ModAct::Remove, ModAct::Remove)
                    | (ModAct::Filter, ModAct::Filter) => Self::merge(this_data, other_data),

                    (ModAct::None, ModAct::Report) => other_data,
                    (ModAct::None, ModAct::Remove) => other_data,
                    (ModAct::None, ModAct::Filter) => other_data,

                    (ModAct::Report, ModAct::None) => this_data,
                    (ModAct::Report, ModAct::Remove) => other_data,
                    (ModAct::Report, ModAct::Filter) => other_data,

                    (ModAct::Remove, ModAct::None) => this_data,
                    (ModAct::Remove, ModAct::Report) => this_data,
                    (ModAct::Remove, ModAct::Filter) => other_data,

                    (ModAct::Filter, ModAct::None) => this_data,
                    (ModAct::Filter, ModAct::Report) => this_data,
                    (ModAct::Filter, ModAct::Remove) => this_data,
                };

                PostAction::Action(data)
            }
        }
    }

    fn merge(this: ActionData, other: ActionData) -> ActionData {
        let module = if this.module.len() == 0 {
            other.module
        } else {
            this.module + "," + other.module.as_str()
        };

        let analyser = match (this.analyser, other.analyser) {
            (None, None) => None,
            (None, Some(t)) | (Some(t), None) => Some(t),
            (Some(mut l), Some(r)) => {
                l.push(',');
                l.push_str(&r);
                Some(l)
            }
        };

        let reply = match (this.reply, other.reply) {
            (None, None) => None,
            (None, Some(reply)) | (Some(reply), None) => Some(reply),
            (Some(mut this), Some(other)) => {
                this.text.push_str("\n------\n");
                this.text.push_str(&other.text);

                this.distinguish = match (this.distinguish, other.distinguish) {
                    (Distinguish::Special, _) | (_, Distinguish::Special) => Distinguish::Special,
                    (Distinguish::Admin, _) | (_, Distinguish::Admin) => Distinguish::Admin,
                    (Distinguish::Moderator, _) | (_, Distinguish::Moderator) => {
                        Distinguish::Moderator
                    }
                    _ => Distinguish::None,
                };

                Some(this)
            }
        };

        ActionData {
            module,
            analyser,
            reply,
            moderate: this.moderate,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionData {
    analyser: Option<String>,
    module: String,
    reply: Option<PostReply>,
    moderate: ModAct,
}

impl ActionData {
    pub fn new() -> Self {
        Self {
            analyser: None,
            module: String::new(),
            reply: None,
            moderate: ModAct::None,
        }
    }

    pub async fn execute(
        self,
        is_debug: bool,
        webhook: Option<&mut WebhookClient>,
        db: &PgClient,
        post: &Submission,
        client: &RouxClient,
    ) -> anyhow::Result<()> {
        let reply_fullname = if let Some(reply) = self.reply {
            let comment = post.comment(&reply.text).await?;
            if reply.distinguish != Distinguish::None {
                comment.distinguish(reply.distinguish, true).await?;
            }

            Some(comment.name().full().to_string())
        } else {
            None
        };

        let (reported, removed) = match self.moderate {
            ModAct::None => (false, false),
            ModAct::Report => {
                if let Some(name) = self.analyser.as_ref() {
                    post.report(&format!("Appears to be a common repost ({name})"))
                        .await
                } else {
                    post.report("Appears to be a common report").await
                }?;

                (true, false)
            }
            ModAct::Remove => {
                post.remove(false).await?;
                (false, true)
            }
            ModAct::Filter => {
                post.remove(false).await?;

                let mut modmail = match self.analyser.as_ref() {
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

                (true, true)
            }
        };

        db.update_item_monitor_state(
            post.name().full(),
            MonitorState::Acted {
                analyzer: self
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
                &self.module,
                self.analyser.as_ref().map(|c| c.as_str()),
                is_debug,
            );
            webhook.send(&msg).await?;
        }

        Ok(())
    }

    fn set_module(&mut self, name: &str) -> &mut Self {
        self.module = name.to_string();
        self
    }

    pub fn module(mut self, name: &str) -> Self {
        self.set_module(name);
        self
    }

    pub fn analyser(mut self, name: &str) -> Self {
        self.analyser = Some(name.to_string());
        self
    }

    pub fn reply(mut self, text: String, distinguish: bool) -> Self {
        self.set_reply(text, distinguish);
        self
    }

    pub fn set_reply(&mut self, text: String, distinguish: bool) -> &mut Self {
        self.reply = Some(PostReply {
            text,
            distinguish: if distinguish {
                Distinguish::Moderator
            } else {
                Distinguish::None
            },
        });
        self
    }

    pub fn set_report(&mut self) -> &mut Self {
        self.moderate = ModAct::Report;
        self
    }

    pub fn report(mut self) -> Self {
        self.set_report();
        self
    }

    pub fn set_remove(&mut self) -> &mut Self {
        self.moderate = ModAct::Remove;
        self
    }

    pub fn remove(mut self) -> Self {
        self.set_remove();
        self
    }

    pub fn set_filter(&mut self) -> &mut Self {
        self.moderate = ModAct::Filter;
        self
    }

    pub fn filter(mut self) -> Self {
        self.set_filter();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostReply {
    text: String,
    distinguish: Distinguish,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ModAct {
    /// Take no moderation decisions
    None,
    /// Report the post
    Report,
    /// Remove the post
    Remove,
    /// Remove the post and send a message to the subreddit's modmail
    Filter,
}

#[cfg(test)]
mod tests {
    use super::{ActionData, PostAction};

    #[test]
    pub fn test_ignore_overriden() {
        let first = PostAction::Ignore;
        let second = PostAction::Action(ActionData::new());

        let result = first.clone().join(second.clone());
        assert_eq!(result, second);

        let result = second.clone().join(first.clone());
        assert_eq!(result, second);
    }

    #[test]
    pub fn test_action_remove_takes_precedence() {
        let first = PostAction::Action(ActionData::new().analyser("first").remove());
        let second = PostAction::Action(
            ActionData::new()
                .analyser("second")
                .reply("second text".into(), true),
        );

        let result = first.clone().join(second.clone());
        assert_eq!(result, first);

        let result = second.clone().join(first.clone());
        assert_eq!(result, first);
    }

    #[test]
    pub fn test_merges_equal() {
        let first = PostAction::Action(
            ActionData::new()
                .analyser("first")
                .module("group")
                .remove()
                .reply("first text".into(), false),
        );

        let second = PostAction::Action(
            ActionData::new()
                .analyser("second")
                .module("parent")
                .remove()
                .reply("second text".into(), true),
        );

        let expected = PostAction::Action(
            ActionData::new()
                .analyser("first,second")
                .module("group,parent")
                .remove()
                .reply("first text\n------\nsecond text".into(), true),
        );

        let result = first.clone().join(second.clone());
        assert_eq!(result, expected);

        let expected = PostAction::Action(
            ActionData::new()
                .analyser("second,first")
                .module("parent,group")
                .remove()
                .reply("second text\n------\nfirst text".into(), true),
        );

        let result = second.join(first);
        assert_eq!(result, expected);
    }
}
