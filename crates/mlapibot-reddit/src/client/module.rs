use roux::{client::AuthedClient, models::LatestComment};

use crate::{
    Comment, RedditMessage, Submission,
    client::ModuleRedditClient,
    config::{SubredditConfig, SubredditsConfig},
    subreddit::Subreddit,
};

pub mod comment_cdn_links;
pub mod comment_code;
pub mod inbox_commands;
pub mod post_flairs;
pub mod post_scams;
pub mod post_vague_title;

pub use comment_cdn_links::CdnLinks;
pub use comment_code::CommentCode;
pub use inbox_commands::InboxCommands;
pub use post_flairs::PostFlairs;
pub use post_scams::PostScams;
pub use post_vague_title::PostVagueTitle;

pub trait Module {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &'static str;
    fn wants(&self) -> ModuleWants;

    fn mask_subreddits(&self, config: &SubredditsConfig, subreddits: &[Subreddit]) -> SplitSubMask {
        SplitSubMask::new()
    }

    fn run_post<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&SubredditConfig>,
        post: &Submission,
        has_seen: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_comment<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        comment: &LatestComment<AuthedClient>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_inbox<'client>(
        &mut self,
        client: &mut ModuleRedditClient<'client>,
        subreddits: &mut [Subreddit],
        msg: &InboxMsg<'_>,
    ) -> anyhow::Result<Option<InboxAction>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleWants(u8);

impl ModuleWants {
    pub const POSTS: ModuleWants = ModuleWants(0b001);
    pub const COMMENTS: ModuleWants = ModuleWants(0b010);
    pub const INBOX: ModuleWants = ModuleWants(0b100);

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
    Redo(Submission),
}

macro_rules! impl_mask_subreddits {
    (
        $flag:ident => $wants:ident
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
                    sum.$wants.set(idx);
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
        let subject = inner.subject().as_str();
        let (subject, body) = if subject == "[direct chat room]" {
            match inner.body().split_once('\n') {
                Some(pair) => pair,
                None => (subject, inner.body().as_str()),
            }
        } else {
            (subject, inner.body().as_str())
        };

        let author = match inner.author() {
            Some(s) => s.as_str(),
            None => "",
        };

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

    fn reply(
        &self,
        content: &str,
    ) -> Result<roux::models::Message<roux::client::AuthedClient>, roux::util::RouxError> {
        self.inner.reply(content)
    }
}
