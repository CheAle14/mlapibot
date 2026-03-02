use anyhow::Context;

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

        let result = match mlapibot_analysis::get_best_analysis(&ctx, client.analzyers) {
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

        if let Some((detection, detected)) = result {
            println!(
                "Triggered on post {:?} by /u/{}",
                post.title(),
                post.author()
            );

            let mut template_context = tera::Context::new();

            let should_remove = if detected.remove && post.moderation().is_some() {
                let reason_id = removal_reasons.get_or_default(&detected.name);

                if let Some(reason_id) = reason_id {
                    let reason = subreddit
                        .removal_reasons
                        .data(&subreddit.reddit)
                        .await?
                        .get(reason_id)
                        .map(|r| r.message.as_str())
                        .unwrap_or("<error: removal reason not found>");

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

            match detected.template.name() {
                Some(text) => {
                    let template = client
                        .templates
                        .render(text, &template_context)
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
