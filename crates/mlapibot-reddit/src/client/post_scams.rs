use anyhow::Context as AnyhowContext;
use mlapibot_datastore::MlapiDb;
use roux::models::Distinguish;
use tera::Tera;

use mlapibot_analysis::{Context, analzyer::Analyzer};
use mlapibot_imgur::ImgurClient;
use mlapibot_webhook::{WebhookClient, create_generic_error_message};

use super::Submission;

use crate::{
    config::SubredditModerateConfig,
    subreddit::Subreddit,
    webhook::{create_detection_message, create_error_processing_post},
};

impl<'a> super::RedditClient<'a> {
    pub(super) fn run_post_scam_checks(
        db: &MlapiDb,
        webhook: &mut Option<WebhookClient>,
        analzyers: &[Analyzer],
        imgur: &mut Option<ImgurClient>,
        modconf: Option<&SubredditModerateConfig>,
        templates: &Tera,
        dry_run: bool,
        subreddit: &mut Subreddit,
        post: &Submission,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        let result = match mlapibot_analysis::get_best_analysis(&ctx, analzyers) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error whilst analyising {}: {err:?}", post.id());
                if let Some(webhook) = webhook {
                    let msg = create_error_processing_post(&post);
                    webhook.send(&msg)?;
                }
                return Ok(());
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
                        .get_removal_reason(reason_id)?
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

            let imgur_link = match (
                detected.template.name().is_some(), // no point uploading images if we aren't replying
                ctx.images.len() > 0,
                imgur.as_mut(),
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
                            if let Some(webhook) = webhook {
                                let _ = webhook.send(&msg);
                            }
                            None
                        }
                    }
                }
                (_, _, _) => None,
            };

            let (reply, removed, reported) = if !dry_run {
                let own_comment = match detected.template.name() {
                    Some(text) => {
                        let template =
                            templates.render(text, &template_context).with_context(|| {
                                format!("rendering to template {:?}", detected.template)
                            })?;

                        let comment = post
                            .comment(&template)
                            .with_context(|| format!("reply to {:?}", post.name()))?;

                        Some(comment)
                    }
                    None => None,
                };

                if is_mod {
                    post.remove(false)?;

                    if let Some(own_comment) = &own_comment {
                        own_comment.distinguish(Distinguish::Moderator, true)?;
                    }
                } else if detected.report {
                    post.report(&format!(
                        "Appears to be a common repost ({})",
                        detected.name
                    ))
                    .with_context(|| format!("report {:?}", post.name()))?;
                }

                if let Some(webhook) = webhook {
                    let msg = create_detection_message(&post, &detection, detected, imgur_link);
                    webhook.send(&msg).context("send detection webhook")?;
                }

                (
                    own_comment.map(|c| c.name().full().to_string()),
                    is_mod,
                    !is_mod && detected.report,
                )
            } else {
                (None, false, false)
            };

            db.set_analyzed(
                post.name().full(),
                &detected.name,
                reply.as_ref().map(|s| s.as_str()),
                reported,
                removed,
            )?;
        } else {
            db.set_ignored(post.name().full())?;
        }

        Ok(())
    }
}
