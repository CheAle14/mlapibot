use mlapibot_common::Words;
use roux::{client::RemoveReason, util::error::RouxErrorKind};

pub struct CommentComplex;

impl super::Module for CommentComplex {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "comment_complex"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::COMMENTS
    }

    super::impl_mask_subreddits!(comments_complex => comments);

    fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        let Some(config) = client.subreddits_config.get(comment.subreddit()) else {
            return Ok(());
        };

        for complex in &config.comments_complex {
            if comment
                .author_flair_css_class()
                .is_some_and(|v| complex.ignore_flairs.iter().any(|f| f == v))
                || comment
                    .author_flair_template_id()
                    .is_some_and(|v| complex.ignore_flairs.iter().any(|f| f == v))
            {
                continue;
            }

            let title_words = Words::new(comment.link_title());
            if !complex.link_title.iter().any(|v| title_words.contains(v)) {
                continue;
            }

            let comment_words = Words::new(comment.body());
            if !complex.comment.iter().any(|v| comment_words.contains(v)) {
                continue;
            }

            comment.remove_with_reason(false, RemoveReason::ReasonId(&complex.reason_id))?;
        }

        Ok(())
    }
}
