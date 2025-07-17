use roux::{client::AuthedClient, models::LatestComment};

use crate::{
    Comment, RedditMessage, Submission,
    client::ModuleRedditClient,
    config::{SubredditConfig, SubredditsConfig},
    subreddit::Subreddit,
};

pub mod comment_code;
pub mod inbox_commands;
pub mod post_flairs;
pub mod post_scams;

pub use comment_code::CommentCode;
pub use inbox_commands::InboxCommands;
pub use post_flairs::PostFlairs;
pub use post_scams::PostScams;

pub trait Module {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &'static str;
    fn wants(&self) -> ModuleWants;

    fn mask_subreddits(&self, config: &SubredditsConfig, subreddits: &[Subreddit]) -> SubMask {
        SubMask::all(subreddits.len())
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
        inbox: &RedditMessage,
        author: &str,
        subject: &str,
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

pub enum InboxAction {
    Redo(Submission),
}
