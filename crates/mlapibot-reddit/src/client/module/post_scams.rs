use anyhow::Context;
use mlapibot_webhook::create_generic_error_message;

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

    super::impl_mask_subreddits!(scams => posts);

    async fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<PostAction> {
        if has_seen {
            return Ok(PostAction::Ignore);
        }

        let modconf = config.map(|c| c.moderate.as_ref()).flatten();

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

            let is_mod = match (modconf, detected.remove) {
                (Some(modconf), true) if post.moderation().is_some() => {
                    let reason_id = modconf
                        .removal_reasons
                        .get(&detected.name)
                        .map(String::as_str)
                        .unwrap_or_else(|| &modconf.default_removal_reason);

                    let reason = subreddit
                        .get_removal_reason(reason_id)
                        .await?
                        .map(|r| r.message.as_str())
                        .unwrap_or("<error: removal reason not found>");

                    template_context.insert("removal_reason", reason);
                    true
                }
                _ => {
                    println!(
                        "Not modding post (mod config? {}, should remove = {} and can_mod is {}",
                        modconf.is_some(),
                        detected.remove,
                        post.moderation().is_some()
                    );
                    false
                }
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

                    action.set_reply(template, is_mod);
                }
                None => (),
            };

            if is_mod {
                action.set_remove();
            } else if detected.report {
                action.set_report();
            }

            Ok(PostAction::Action(action))
        } else {
            Ok(PostAction::Ignore)
        }
    }
}
