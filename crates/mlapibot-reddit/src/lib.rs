mod client;
pub mod config;
pub mod exts;
mod flairs;
mod ratelimiter;
mod status_tracker;
mod subreddit;
mod utils;
mod webhook;

pub use client::RedditClient;

pub type RouxClient = roux::client::AuthedClient;
pub type Submission = roux::models::Submission<RouxClient>;
pub type Comment = roux::models::ArticleComment<RouxClient>;
pub type RedditMessage = roux::models::Message<RouxClient>;
pub type CreatedComment = roux::models::CreatedComment<RouxClient>;
pub type CreatedCommentWithLinkInfo = roux::models::CreatedCommentWithLinkInfo<RouxClient>;

#[derive(Debug)]
pub struct QuickStopError;

impl std::fmt::Display for QuickStopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("quickly stopping the bot")
    }
}

impl std::error::Error for QuickStopError {}
