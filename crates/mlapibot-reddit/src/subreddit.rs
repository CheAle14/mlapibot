use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Context;
use chrono::Utc;

use mlapibot_database_v2::repos::{
    incidents::{IncidentRepo, ResolvedIncidentPost, StickyState},
    subreddits::{ReplyTemplate, ReplyTemplateId, Scam, StatusModule, StatusStickyConfig},
};
use roux::{
    api::{ThingFullname, subreddit::RemovalReason},
    client::RedditClient,
    models::SubmissionStickySlot,
    util::RouxError,
};

use mlapibot_common::{LazyCached, LowercaseString};

use crate::{client::module::post_scams::ScamAnalyzer, status_tracker::IncidentWithLive};

use super::{RouxClient, Submission};

pub type RouxSubreddit = roux::client::Subreddit<super::RouxClient>;
pub type DbSubreddit = mlapibot_database_v2::repos::subreddits::Subreddit;

type SubCached<T> = LazyCached<T, RouxSubreddit, RouxError>;

pub struct Subreddit {
    pub reddit: RouxSubreddit,
    pub db: DbSubreddit,
    lower: LowercaseString,
    // Cached in the database, periodically refreshed per
    // the `db.lasy_sync` time.
    moderators: HashSet<String>,

    pub template_map: HashMap<ReplyTemplateId, String>,
    pub templates: tera::Tera,
    pub analyzers: Vec<ScamAnalyzer>,

    pub removal_reasons: SubCached<HashMap<String, RemovalReason>>,
}

fn minimal_urldecode(text: &mut String) {
    const GT: &str = "&gt;";

    while let Some(idx) = text.find(GT) {
        text.replace_range(idx..(idx + GT.len()), ">");
    }
}

async fn fetch_removal_reasons(
    ctx: &RouxSubreddit,
) -> Result<HashMap<String, RemovalReason>, RouxError> {
    let mut map = ctx.list_removal_reasons().await?.data;

    for value in map.values_mut() {
        minimal_urldecode(&mut value.message);
    }

    Ok(map)
}

impl Subreddit {
    pub async fn new(
        reddit: RouxSubreddit,
        db: DbSubreddit,
        templates: Vec<ReplyTemplate>,
        analyzers: Vec<Scam>,
        moderators: HashSet<String>,
        name: LowercaseString,
    ) -> anyhow::Result<Self> {
        let removal_reasons = SubCached::new(Duration::from_secs(60 * 60), |c| {
            Box::pin(fetch_removal_reasons(c))
        });

        let mut tera = tera::Tera::default();
        let mut map = HashMap::new();

        tera.add_raw_templates(
            templates
                .iter()
                .map(|t| (t.name.as_str(), t.content.as_str())),
        )
        .with_context(|| format!("build tera templates for /r/{name}"))?;

        for template in templates {
            map.insert(template.id, template.name);
        }

        Ok(Self {
            reddit,
            db,
            lower: name,
            templates: tera,
            template_map: map,
            analyzers: analyzers.into_iter().map(ScamAnalyzer).collect(),
            removal_reasons,
            moderators,
        })
    }

    pub fn is_moderator(&mut self, username: &str) -> bool {
        self.moderators.contains(username)
    }

    pub fn name(&self) -> &LowercaseString {
        &self.lower
    }

    pub async fn sticky_incident_post<DB: IncidentRepo>(
        reddit: &mut RouxSubreddit,
        db: &DB,
        sticky: &StatusStickyConfig,
        submission: &Submission,
    ) -> anyhow::Result<()> {
        let prior_id = if let Some(replace) = sticky.replace_sticky.as_ref() {
            let replacing = Self::get_sticky_to_replace(reddit, replace).await?;

            if let Some(replacing) = replacing.as_ref() {
                println!("replacing {:?}", replacing.name());
                // slot does not matter here.
                replacing.sticky(false, SubmissionStickySlot::Top).await?;
            } else {
                println!("replacing nothing??");
            };

            replacing
        } else {
            None
        };

        submission
            .sticky(true, SubmissionStickySlot::Bottom)
            .await?;

        let prior_id = prior_id.as_ref().map(|f| f.name().full());
        println!("Stickying with prior unsticky: {:?}", prior_id);

        db.sticky_incident_post(submission.name().full(), prior_id)
            .await?;

        Ok(())
    }

    fn is_incident_major(incident: &IncidentWithLive, config: &StatusModule) -> bool {
        let Some(sticky) = &config.sticky else {
            return false;
        };

        if let Some(only_for) = sticky.only_for.as_ref()
            && only_for.len() > 0
        {
            let has_components = (&incident.incident.components)
                .iter()
                .any(|c| only_for.contains(&c.id) || only_for.contains(&c.name));

            if !has_components {
                println!("Incident does not affect components required to sticky");
                return false;
            }
        }

        if let Some(min_impact) = sticky.min_impact {
            if incident.incident.impact < min_impact {
                println!("Incident does not meet minimum {min_impact:?} to sticky");
                return false;
            }
        }

        true
    }

    async fn send_incident_post<DB: IncidentRepo>(
        &mut self,
        db: &DB,
        incident: &IncidentWithLive<'_>,
        reddit: &RouxClient,
    ) -> anyhow::Result<()> {
        println!("Sending incident to /r/{}", self.lower);

        let is_major = Self::is_incident_major(incident, &self.db.mod_status);

        // TODO: re-add flairing.
        // let submission = match &config.flair {
        //     Some(flair) => incident.to_builder().with_flair_setting(if is_major {
        //         flair.major.as_ref().unwrap_or(&flair.minor)
        //     } else {
        //         &flair.minor
        //     }),
        //     None => incident.to_builder(),
        // };

        let submission = incident.to_builder();

        let submission = reddit.submit(&self.lower.as_str(), &submission).await?;
        println!("Incident posted as {:?}", submission.name());

        db.create_incident_post(
            self.lower.as_str(),
            &incident.incident.id,
            submission.name().full(),
        )
        .await?;

        if self.db.mod_status.distinguish {
            submission
                .distinguish(roux::models::Distinguish::Moderator)
                .await?;
        }

        if is_major {
            Self::sticky_incident_post(
                &mut self.reddit,
                db,
                self.db
                    .mod_status
                    .sticky
                    .as_ref()
                    .expect("is_major only true if sticky config there"),
                &submission,
            )
            .await?;
        }

        Ok(())
    }

    async fn get_sticky_to_replace(
        subreddit: &mut RouxSubreddit,
        replace: &str,
    ) -> anyhow::Result<Option<Submission>> {
        println!("Looking for {replace:?}");
        let Some(top) = subreddit.sticky(SubmissionStickySlot::Top).await? else {
            println!("No top sticky post");
            return Ok(None);
        };

        if let Some(id) = top.link_flair_template_id() {
            if id.as_str() == replace {
                return Ok(Some(top));
            }
        }
        println!(
            "Top template no match, was: {:?}",
            top.link_flair_template_id()
        );

        let Some(bottom) = subreddit.sticky(SubmissionStickySlot::Bottom).await? else {
            println!("No bottom sticky post");
            return Ok(None);
        };

        if let Some(id) = bottom.link_flair_template_id() {
            if id.as_str() == replace {
                return Ok(Some(bottom));
            }
        }

        println!(
            "Bottom template no match, was: {:?}",
            bottom.link_flair_template_id()
        );

        Ok(None)
    }

    async fn set_resolved_flair(_post: &Submission, _config: &StatusModule) -> anyhow::Result<()> {
        // TODO: re-add flairing
        // let Some(flair) = config.flair.as_ref() else {
        //     return Ok(());
        // };

        // let Some(resolved) = flair.resolved.as_ref() else {
        //     return Ok(());
        // };

        // post.select_flair(&resolved.as_update()).await?;

        Ok(())
    }

    pub async fn check_incident_sticky<DB: IncidentRepo>(
        &mut self,
        db: &DB,
        post: ResolvedIncidentPost,
    ) -> anyhow::Result<()> {
        if post.sticky_state == StickyState::NeverStickied {
            println!("Never stickied!");
            return Ok(());
        }

        let Some(sticky) = self.db.mod_status.sticky.as_ref() else {
            println!("No sticky config");
            return Ok(());
        };

        let current_time = (Utc::now() - post.resolved_at).num_minutes();

        if current_time < sticky.delay_minor_mins as i64 {
            // definitely not ready yet, even if comments are under threshold.
            println!("Not reached the minor delay threshold mins");
            return Ok(());
        }

        let Ok(thing) = ThingFullname::try_from(post.post_fullname) else {
            println!("Not a valid fullname");
            return Ok(());
        };

        let submission: Submission = self
            .reddit
            .client
            .get_submissions(&[&thing])
            .await?
            .into_iter()
            .next()
            .unwrap();

        if !submission.stickied() {
            // assume that a human mod has unstickied it manually.
            println!("Already unstickied");
            db.unsticky_incident_post(thing.full(), Utc::now()).await?;
            Self::set_resolved_flair(&submission, &self.db.mod_status).await?;
            return Ok(());
        }

        let delay = if submission.num_comments() < sticky.comment_threshold {
            sticky.delay_minor_mins
        } else {
            sticky.delay_major_mins
        };

        if current_time < delay as i64 {
            println!(
                "Not yet reached delay of {delay} ({} comments / {})",
                submission.num_comments(),
                sticky.comment_threshold
            );
            return Ok(());
        }

        match post.sticky_state {
            StickyState::Stickied {
                removed: Some(put_back),
            } => {
                let bot_slot = self.reddit.sticky(SubmissionStickySlot::Bottom).await?;

                submission.unsticky().await?;

                let put_back = self
                    .reddit
                    .client
                    .get_submissions(&[&ThingFullname::try_from(put_back).expect("was fullname")])
                    .await?
                    .into_iter()
                    .next()
                    .unwrap();

                println!("wanting to put back {}", put_back.title());

                match bot_slot {
                    None => {
                        println!("bottom slot was empty");
                        // there was only one sticky, which was our post.
                        // so we can just sticky this back
                        put_back
                            .sticky(true, roux::models::SubmissionStickySlot::Top)
                            .await?;
                    }
                    Some(slot) => {
                        println!("bottom slot was {}", slot.title());
                        // there *were* two stickies.
                        let not_our_post = if slot.name() == submission.name() {
                            // the bottom slot was our post, so the other one is in the top slot.
                            // unfortunately, that means we'll have to fetch it
                            self.reddit
                                .sticky(SubmissionStickySlot::Top)
                                .await?
                                .expect("has other top sticky")
                        } else {
                            // the top slot was our post, so this bottom one is the other one
                            slot
                        };

                        println!("top slot was {}", not_our_post.title());

                        // Sticky this back onto the top.
                        put_back
                            .sticky(true, roux::models::SubmissionStickySlot::Top)
                            .await?;
                        // That will have unstickied the other post, so re-sticky that to the bottom
                        not_our_post
                            .sticky(true, SubmissionStickySlot::Bottom)
                            .await?;
                    }
                }
            }
            other => {
                eprintln!("unexpected sticky state: {other:?}");
            }
        }

        db.unsticky_incident_post(thing.full(), Utc::now()).await?;
        Self::set_resolved_flair(&submission, &self.db.mod_status).await?;

        Ok(())
    }

    async fn check_posts_for_unsticky<DB: IncidentRepo>(&mut self, db: &DB) -> anyhow::Result<()> {
        for post in db.get_resolved_incidents_still_stickied().await? {
            self.check_incident_sticky(db, post).await?;
        }

        Ok(())
    }

    pub async fn update_status<DB: IncidentRepo>(
        &mut self,
        db: &DB,
        reddit: &RouxClient,
        updated_incidents: &[IncidentWithLive<'_>],
        is_summary: bool,
    ) -> anyhow::Result<HashSet<String>> {
        self.check_posts_for_unsticky(db)
            .await
            .context("check unsticky")?;

        let mut unseen = db
            .get_stickied_incident_posts(self.lower.as_str())
            .await
            .context("get unresolved")?;

        for update in updated_incidents {
            if let Some(_) = db
                .get_incident_post(self.lower.as_str(), update.incident.id.as_str())
                .await?
            {
                unseen.remove(&update.incident.id);
            } else if update.incident.impact >= self.db.mod_status.min_impact {
                self.send_incident_post(db, update, reddit).await?;
            }
        }

        if !is_summary {
            // from a webhook, so it is expected that other incidents are missing
            return Ok(HashSet::new());
        }

        Ok(unseen)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test_removal_decode() {
        const BEFORE: &str = "&gt; hello &gt;&gt;&gt; more text &gt;here";

        const AFTER: &str = "> hello >>> more text >here";

        let mut text = String::from(BEFORE);
        super::minimal_urldecode(&mut text);

        assert_eq!(text, AFTER);
    }
}
