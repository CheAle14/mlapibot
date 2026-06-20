use mlapibot_common::Words;
use roux::{
    client::RemoveReason,
    util::{RouxError, error::RouxErrorKind},
};

use crate::subreddit::Subreddit;

pub struct CommentComplex;

#[async_trait::async_trait(?Send)]
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

    super::impl_mask_subreddits!(mod_complex_comments => comments);

    async fn run_comment<'client>(
        &mut self,
        _client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut Subreddit,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        let config = &subreddit.db.mod_complex_comments;

        for complex in &config.items {
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

            match comment
                .remove_with_reason(false, RemoveReason::ReasonId(&complex.reason))
                .await
            {
                Ok(_) => (),
                Err(RouxError {
                    kind: RouxErrorKind::RedditError2(error),
                    ..
                }) if error.reason == "INVALID_ID" => {
                    eprintln!(
                        "Failed to remove with reason post={} comment={} reason={:?}",
                        comment.link_id().id(),
                        comment.id(),
                        complex.reason
                    );
                }
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }
}
