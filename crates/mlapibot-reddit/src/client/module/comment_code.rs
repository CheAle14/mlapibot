use roux::{builders::submission::SubmissionSubmitBuilder, client::RedditClient};

use crate::client::module::{Module, SplitSubMask, SubMask};

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

    super::impl_mask_subreddits!(comments_code => comments);

    fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        use std::fmt::Write;

        let fences = mlapibot_markdown::extract_code_fences(comment.body().as_str());

        let all_small = fences.iter().all(|fence| {
            let mut lines = 0;
            let mut any_long = false;

            for line in fence.code.lines() {
                lines += 1;
                if line.len() > 16 {
                    any_long = true;
                }
            }

            // Vertically and horizontally small.
            lines <= 3 && !any_long
        });

        if fences.len() == 0 || all_small {
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
