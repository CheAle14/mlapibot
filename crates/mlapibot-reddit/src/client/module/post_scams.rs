use anyhow::Context;
use mlapibot_analysis::analzyer::Analyzer;
use mlapibot_database_v2::repos::subreddits::Scam;

use crate::{
    RedditClient,
    client::module::{ActionData, PostAction},
    exts::SubmissionExt,
    webhook::create_error_processing_post,
};

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

        let removal_reasons = &subreddit.db.removal_reasons;

        let mut warnings = Vec::new();
        let ctx = mlapibot_analysis::Context::new_submission(
            post.get_misc_links().into_iter(),
            post.title(),
            post.selftext(),
            &mut warnings,
        )
        .await?;

        if warnings.len() > 0 {
            RedditClient::_send_warnings(
                client.webhook.as_mut(),
                warnings,
                format!("Warnings with post {:?}, {:?}", post.id(), post.permalink()),
            )
            .await?;
        }

        let result = match mlapibot_analysis::get_best_analysis(&ctx, &subreddit.analyzers) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error whilst analyising {}: {err:?}", post.id());
                if let Some(webhook) = client.webhook {
                    let msg = create_error_processing_post(&post);
                    webhook.send(&msg).await?;
                }
                return Ok(PostAction::Ignore);
            }
        };

        if let Some((_detection, detected)) = result {
            let detected = &detected.0;
            println!(
                "Triggered on post {:?} by /u/{}",
                post.title(),
                post.author()
            );

            let mut template_context = tera::Context::new();

            let should_remove = if detected.remove && post.moderation().is_some() {
                let reason_id = detected
                    .reason
                    .as_ref()
                    .and_then(|key| removal_reasons.get(key));

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
                            for (key, mapping) in removal_reasons {
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

            #[cfg(feature = "imgur")]
            match (
                detected.template.name().is_some(), // no point uploading images if we aren't replying
                ctx.images.len() > 0,
                client.imgur.as_mut(),
            ) {
                (true, true, Some(imgur)) => {
                    match mlapibot_imgur::upload_images(imgur, ctx.images.iter(), |idx| {
                        detection
                            .images
                            .get(&idx)
                            .map(|d| ctx.images[idx].get_trigger_words_image(d))
                            .flatten()
                    }) {
                        Ok(album) => {
                            let url = format!("https://imgur.com/a/{}", album.id);
                            template_context.insert("imgur_url", &url);
                            Some(url)
                        }
                        Err(e) => {
                            let msg = create_generic_error_message(
                                "Uploading to imgur",
                                format!("{e:?}"),
                            );
                            if let Some(webhook) = client.webhook {
                                let _ = webhook.send(&msg);
                            }
                            None
                        }
                    }
                }
                (_, _, _) => None,
            };

            match detected
                .template
                .as_ref()
                .and_then(|id| subreddit.template_map.get(id))
            {
                Some(template) => {
                    let template = subreddit
                        .templates
                        .render(&template, &template_context)
                        .with_context(|| {
                            format!("rendering to template {:?}", detected.template)
                        })?;

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
