use roux::{builders::submission::SubmissionSubmitBuilder, client::RedditClient};

use crate::client::module::{Module, SubMask};

pub struct CommentCode;

impl Module for CommentCode {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "comment_code"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::COMMENTS
    }

    fn mask_subreddits(
        &self,
        config: &crate::config::SubredditsConfig,
        subreddits: &[crate::subreddit::Subreddit],
    ) -> SubMask {
        let mut sum = SubMask::new();
        for (idx, sub) in subreddits.iter().enumerate() {
            if config
                .get(sub.name())
                .map(|c| c.comments_code)
                .unwrap_or_default()
            {
                sum.set(idx);
            }
        }

        sum
    }

    fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        use std::fmt::Write;

        let fences = mlapibot_markdown::extract_code_fences(comment.body().as_str());

        if fences.len() == 0 {
            return Ok(());
        }

        let mut text = format!(
            "For {} backtick-style code blocks in old Reddit format:",
            comment.permalink()
        );

        for fence in fences {
            let _ = writeln!(text, "\n\n{fence}\n---");
        }

        client
            .client
            .subreddit("mlapi")
            .submit(&SubmissionSubmitBuilder::text("Fence conversion", text))?;

        Ok(())
    }
}
