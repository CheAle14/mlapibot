use anyhow::Context;
use mlapibot_analysis::{ContextWarning, Url, analzyer::Analyzer};
use mlapibot_common::action::{ActionData, PostAction};
use mlapibot_database_v2::repos::subreddits::Scam;

use crate::{RedditClient, exts::SubmissionExt, subreddit::Subreddit};

pub struct PostScams;

#[async_trait::async_trait(?Send)]
impl super::Module for PostScams {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "post_scams"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::POSTS
    }

    super::impl_mask_subreddits!(mod_scams => posts);

    async fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        if has_seen {
            return Ok(PostAction::Ignore);
        }

        let links = post.get_misc_links();

        return analyze_post(
            client,
            subreddit,
            post.title(),
            links.into_iter(),
            post.selftext().as_str(),
            post.moderation().is_some(),
        )
        .await;
    }
}

pub async fn analyze_post<R: Reporter>(
    reporter: &mut R,
    subreddit: &mut Subreddit,
    title: &str,
    links: impl Iterator<Item = Url> + ExactSizeIterator,
    body: &str,
    can_moderate: bool,
) -> anyhow::Result<PostAction> {
    let mut warnings = Vec::new();
    let ctx = mlapibot_analysis::Context::new_submission(links, title, body, &mut warnings).await?;

    if warnings.len() > 0 {
        reporter.image_warnings(title, warnings).await?;
    }

    let result = match mlapibot_analysis::get_best_analysis(&ctx, &subreddit.analyzers) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error whilst analyising {title}: {err:?}");
            return Ok(PostAction::Ignore);
        }
    };

    if let Some((_detection, detected)) = result {
        let detected = &detected.0;

        let mut template_context = tera::Context::new();

        let should_remove = if detected.remove && can_moderate {
            let reason_id = detected
                .reason
                .as_ref()
                .and_then(|key| subreddit.db.removal_reasons.get(key));

            if let Some(reason_id) = reason_id {
                let all_reasons = subreddit.removal_reasons.data(&subreddit.reddit).await?;

                let reason = match all_reasons.get(reason_id).map(|r| r.message.as_str()) {
                    Some(reason) => reason,
                    None => {
                        eprintln!(
                            "failed to get removal reason {reason_id:?} from {:?}",
                            detected.reason
                        );

                        eprintln!("reddit has:");
                        for (id, reason) in all_reasons {
                            eprintln!("- {} = {}", id, reason.title);
                        }

                        eprintln!("\nour map is:");
                        for (key, mapping) in &subreddit.db.removal_reasons {
                            eprintln!("- {key} -> {mapping}");
                        }

                        "<error: removal reason not found>"
                    }
                };

                template_context.insert("removal_reason", reason);
            }

            true
        } else {
            false
        };

        let mut action = ActionData::new().analyser(&detected.name);

        match detected
            .template
            .as_ref()
            .and_then(|id| subreddit.template_map.get(id))
        {
            Some(template) => {
                let template = subreddit
                    .templates
                    .render(&template, &template_context)
                    .with_context(|| format!("rendering to template {:?}", detected.template))?;

                action.set_reply(template, should_remove);
            }
            None => (),
        };

        if should_remove {
            if detected.report {
                // remove + report = filter
                // ideally we would report like /u/AutoModerator, by
                // sending it to the modqueue. Unfortunately we can't,
                // so we just send to modmail instead.
                action.set_filter();
            } else {
                action.set_remove();
            }
        } else if detected.report {
            action.set_report();
        }

        Ok(PostAction::Action(action))
    } else {
        Ok(PostAction::Ignore)
    }
}

pub trait Reporter {
    async fn image_warnings(
        &mut self,
        title: &str,
        warnings: Vec<ContextWarning>,
    ) -> anyhow::Result<()>;
}

impl Reporter for crate::client::ModuleRedditClient<'_> {
    async fn image_warnings(
        &mut self,
        title: &str,
        warnings: Vec<ContextWarning>,
    ) -> anyhow::Result<()> {
        RedditClient::_send_warnings(
            self.webhook.as_mut(),
            warnings,
            format!("Warnings with post {:?}", title),
        )
        .await
    }
}

impl Reporter for () {
    async fn image_warnings(
        &mut self,
        title: &str,
        warnings: Vec<ContextWarning>,
    ) -> anyhow::Result<()> {
        eprintln!("Warnings when analyzing {title}:");

        for warning in warnings {
            eprintln!("- {warning}");
        }

        Ok(())
    }
}

pub struct ScamAnalyzer(pub Scam);

impl std::fmt::Debug for ScamAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Analyzer for ScamAnalyzer {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn ocr(&self) -> Option<&mlapibot_common::matchers::Matchers> {
        self.0.ocr.as_ref()
    }

    fn title(&self) -> Option<&mlapibot_common::matchers::Matchers> {
        self.0.title.as_ref()
    }

    fn body(&self) -> Option<&mlapibot_common::matchers::Matchers> {
        self.0.body.as_ref()
    }

    fn title_or_body(&self) -> Option<&mlapibot_common::matchers::Matchers> {
        self.0.title_or_body.as_ref()
    }
}
