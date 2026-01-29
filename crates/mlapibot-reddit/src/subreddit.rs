use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Context;
use chrono::Utc;
use mlapibot_datastore::{
    MlapiDb,
    incident_posts::{ResolvedIncidentPost, StickyState},
};
use roux::{
    api::{FlairId, ThingFullname, moderator::ModeratorData, subreddit::RemovalReason},
    client::{RedditClient, SelectFlairData},
    models::SubmissionStickySlot,
    util::{FeedOption, RouxError},
};

use mlapibot_common::{LazyCached, LowercaseString};

use crate::{
    config::{FlairSubBuilderExt, StatusStickyConfig, SubredditStatusConfig},
    status_tracker::IncidentWithLive,
};

use super::{RouxClient, Submission};

pub type RouxSubreddit = roux::client::Subreddit<super::RouxClient>;

type SubCached<T> = LazyCached<T, RouxSubreddit, RouxError>;

pub struct Subreddit {
    pub data: RouxSubreddit,
    lower: LowercaseString,
    moderators: SubCached<Vec<ModeratorData>>,
    removal_reasons: SubCached<HashMap<String, RemovalReason>>,
}

fn minimal_urldecode(text: &mut String) {
    const GT: &str = "&gt;";

    while let Some(idx) = text.find(GT) {
        text.replace_range(idx..(idx + GT.len()), ">");
    }
}

fn fetch_removal_reasons(ctx: &RouxSubreddit) -> Result<HashMap<String, RemovalReason>, RouxError> {
    let mut map = ctx.list_removal_reasons()?.data;

    for value in map.values_mut() {
        minimal_urldecode(&mut value.message);
    }

    Ok(map)
}

impl Subreddit {
    pub fn new(data: RouxSubreddit, name: LowercaseString) -> anyhow::Result<Self> {
        let removal_reasons = SubCached::new(Duration::from_secs(60 * 60), fetch_removal_reasons);

        let moderators = SubCached::new(Duration::from_secs(15 * 60), |ctx| {
            ctx.moderators().map(|d| d.data.children)
        });

        Ok(Self {
            data,
            lower: name,
            removal_reasons,
            moderators: moderators,
        })
    }

    pub fn is_moderator(&mut self, username: &str) -> Result<bool, RouxError> {
        self.moderators
            .data(&self.data)
            .map(|ls| ls.iter().any(|m| m.name == username))
    }

    pub fn name(&self) -> &LowercaseString {
        &self.lower
    }

    pub fn sticky_incident_post(
        &mut self,
        db: &MlapiDb,
        sticky: &StatusStickyConfig,
        submission: &Submission,
    ) -> anyhow::Result<()> {
        let prior_id = if let Some(replace) = sticky.replace_sticky.as_ref() {
            let replacing = Self::get_sticky_to_replace(&mut self.data, replace)?;

            if let Some(replacing) = replacing.as_ref() {
                println!("replacing {:?}", replacing.name());
                // slot does not matter here.
                replacing.sticky(false, SubmissionStickySlot::Top)?;
            } else {
                println!("replacing nothing??");
            };

            replacing
        } else {
            None
        };

        submission.sticky(true, SubmissionStickySlot::Bottom)?;

        let prior_id = prior_id.as_ref().map(|f| f.name().full());
        println!("Stickying with prior unsticky: {:?}", prior_id);

        db.sticky_incident_post(submission.name().full(), prior_id)?;

        Ok(())
    }

    fn is_incident_major(incident: &IncidentWithLive, config: &SubredditStatusConfig) -> bool {
        let Some(sticky) = &config.sticky else {
            return false;
        };

        if sticky.only_for.len() > 0 {
            let has_components = (&incident.incident.components)
                .iter()
                .any(|c| sticky.only_for.contains(&c.id) || sticky.only_for.contains(&c.name));

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

    fn send_incident_post(
        &mut self,
        db: &MlapiDb,
        incident: &IncidentWithLive,
        reddit: &RouxClient,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<()> {
        println!("Sending incident to /r/{}", self.lower);

        let is_major = Self::is_incident_major(incident, config);

        let submission = match &config.flair {
            Some(flair) => incident.to_builder().with_flair_setting(if is_major {
                flair.major.as_ref().unwrap_or(&flair.minor)
            } else {
                &flair.minor
            }),
            None => incident.to_builder(),
        };

        let submission = reddit.submit(&self.lower.as_str(), &submission)?;
        println!("Incident posted as {:?}", submission.name());

        db.add_incident(
            self.lower.as_str(),
            &incident.incident.id,
            submission.name().full(),
        )?;

        if config.distinguish {
            submission.distinguish(roux::models::Distinguish::Moderator)?;
        }

        if is_major {
            self.sticky_incident_post(
                db,
                config
                    .sticky
                    .as_ref()
                    .expect("must have sticky config to sticky"),
                &submission,
            )?;
        }

        Ok(())
    }

    fn get_sticky_to_replace(
        subreddit: &mut RouxSubreddit,
        replace: &FlairId,
    ) -> anyhow::Result<Option<Submission>> {
        println!("Looking for {replace:?}");
        let Some(top) = subreddit.sticky(SubmissionStickySlot::Top)? else {
            println!("No top sticky post");
            return Ok(None);
        };

        if let Some(id) = top.link_flair_template_id() {
            if id == replace {
                return Ok(Some(top));
            }
        }
        println!(
            "Top template no match, was: {:?}",
            top.link_flair_template_id()
        );

        let Some(bottom) = subreddit.sticky(SubmissionStickySlot::Bottom)? else {
            println!("No bottom sticky post");
            return Ok(None);
        };

        if let Some(id) = bottom.link_flair_template_id() {
            if id == replace {
                return Ok(Some(bottom));
            }
        }

        println!(
            "Bottom template no match, was: {:?}",
            bottom.link_flair_template_id()
        );

        Ok(None)
    }

    fn set_resolved_flair(post: &Submission, config: &SubredditStatusConfig) -> anyhow::Result<()> {
        let Some(flair) = config.flair.as_ref() else {
            return Ok(());
        };

        let Some(resolved) = flair.resolved.as_ref() else {
            return Ok(());
        };

        post.select_flair(&resolved.as_update())?;

        Ok(())
    }

    pub fn check_incident_sticky(
        &mut self,
        db: &MlapiDb,
        post: ResolvedIncidentPost,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<()> {
        if post.sticky_state == StickyState::NeverStickied {
            println!("Never stickied!");
            return Ok(());
        }

        let Some(sticky) = config.sticky.as_ref() else {
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

        let submission = self
            .data
            .client
            .get_submissions(&[&thing])?
            .into_iter()
            .next()
            .unwrap();

        if !submission.stickied() {
            // assume that a human mod has unstickied it manually.
            println!("Already unstickied");
            db.set_incident_post_unstickied(thing.full())?;
            Self::set_resolved_flair(&submission, config)?;
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
                let bot_slot = self.data.sticky(SubmissionStickySlot::Bottom)?;

                submission.unsticky()?;

                let put_back = self
                    .data
                    .client
                    .get_submissions(&[&ThingFullname::try_from(put_back).expect("was fullname")])?
                    .into_iter()
                    .next()
                    .unwrap();

                println!("wanting to put back {}", put_back.title());

                match bot_slot {
                    None => {
                        println!("bottom slot was empty");
                        // there was only one sticky, which was our post.
                        // so we can just sticky this back
                        put_back.sticky(true, roux::models::SubmissionStickySlot::Top)?;
                    }
                    Some(slot) => {
                        println!("bottom slot was {}", slot.title());
                        // there *were* two stickies.
                        let not_our_post = if slot.name() == submission.name() {
                            // the bottom slot was our post, so the other one is in the top slot.
                            // unfortunately, that means we'll have to fetch it
                            self.data
                                .sticky(SubmissionStickySlot::Top)?
                                .expect("has other top sticky")
                        } else {
                            // the top slot was our post, so this bottom one is the other one
                            slot
                        };

                        println!("top slot was {}", not_our_post.title());

                        // Sticky this back onto the top.
                        put_back.sticky(true, roux::models::SubmissionStickySlot::Top)?;
                        // That will have unstickied the other post, so re-sticky that to the bottom
                        not_our_post.sticky(true, SubmissionStickySlot::Bottom)?;
                    }
                }
            }
            other => {
                eprintln!("unexpected sticky state: {other:?}");
            }
        }

        db.set_incident_post_unstickied(thing.full())?;
        Self::set_resolved_flair(&submission, config)?;

        Ok(())
    }

    fn check_posts_for_unsticky(
        &mut self,
        db: &MlapiDb,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<()> {
        for post in db.get_incident_posts_waiting_unsticky()? {
            self.check_incident_sticky(db, post, config)?;
        }

        Ok(())
    }

    pub fn update_status(
        &mut self,
        db: &MlapiDb,
        reddit: &RouxClient,
        updated_incidents: &[IncidentWithLive<'_>],
        is_summary: bool,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<HashSet<String>> {
        self.check_posts_for_unsticky(db, config)
            .context("check unsticky")?;

        let mut unseen = db
            .get_unresolved_incident_posts(self.lower.as_str())
            .context("get unresolved")?;

        for update in updated_incidents {
            if let Some(_) =
                db.get_incident_post(self.lower.as_str(), update.incident.id.as_str())?
            {
                unseen.remove(&update.incident.id);
            } else if update.incident.impact >= config.min_impact {
                self.send_incident_post(db, update, reddit, &config)?;
            }
        }

        if !is_summary {
            // from a webhook, so it is expected that other incidents are missing
            return Ok(HashSet::new());
        }

        Ok(unseen)
    }

    pub fn newest_unseen(&mut self) -> anyhow::Result<Vec<Submission>> {
        let options = FeedOption::new().limit(25);

        let data = self.data.latest(Some(options))?;
        let mut children = data.children;
        children.reverse();

        Ok(children)
    }

    pub fn get_removal_reason(&mut self, id: &str) -> Result<Option<&RemovalReason>, RouxError> {
        self.removal_reasons.data(&self.data).map(|map| map.get(id))
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
