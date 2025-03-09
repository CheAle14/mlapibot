use std::{collections::HashMap, time::Duration};

use anyhow::Context;
use mlapibot_datastore::{MlapiDb, incident_posts::IncidentPostLite};
use roux::{
    api::{ThingFullname, moderator::ModeratorData, subreddit::RemovalReason},
    util::{FeedOption, RouxError},
};
use statuspage::{StatusClient, incident::Incident};

use mlapibot_common::{Cached, LowercaseString};

use crate::{cached_submission::CachedSubmission, config::SubredditStatusConfig};

use super::{RouxClient, Submission, status_tracker::CachedIncidentSubmissions};

pub type RouxSubreddit = roux::client::Subreddit<super::RouxClient>;

type SubCached<T> = Cached<T, RouxSubreddit, RouxError>;

pub struct Subreddit {
    data: RouxSubreddit,
    lower: LowercaseString,
    moderators: SubCached<Vec<ModeratorData>>,
    removal_reasons: SubCached<HashMap<String, RemovalReason>>,
    // whether we are only using this subreddit to send status info
    pub status_only: bool,
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
    pub fn new(
        status_only: bool,
        data: RouxSubreddit,
        name: LowercaseString,
    ) -> anyhow::Result<Self> {
        let removal_reasons =
            Cached::new(Duration::from_secs(60 * 60), &data, fetch_removal_reasons)
                .context("init cache removal reasons")?;

        let moderators = Cached::new(Duration::from_secs(15 * 60), &data, |ctx| {
            ctx.moderators().map(|d| d.data.children)
        })
        .context("init moderators")?;

        Ok(Self {
            data,
            lower: name,
            status_only,
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

    fn send_incident_post(
        &mut self,
        db: &MlapiDb,
        incident_id: &str,
        reddit: &RouxClient,
        flair_id: Option<&str>,
        cached: &CachedSubmission,
    ) -> anyhow::Result<()> {
        println!("Sending incident to /r/{}", self.lower);

        let submission = match flair_id {
            Some(flair_id) => cached.to_builder().with_flair_id(flair_id),
            None => cached.to_builder(),
        };

        let submission = reddit.submit(&self.lower.as_str(), &submission)?;
        println!("Incident posted as {:?}", submission.name());

        db.add_incident(
            self.lower.as_str(),
            incident_id,
            submission.name().full(),
            cached.get_hash(),
        )?;

        Ok(())
    }

    fn update_incident_post(
        &self,
        db: &MlapiDb,
        tracked: &IncidentPostLite,
        cached: &CachedSubmission,
        reddit: &RouxClient,
    ) -> anyhow::Result<()> {
        if tracked.body_hash != cached.get_hash() {
            reddit.edit(
                cached.get_body(),
                &ThingFullname::try_from(tracked.post_fullname.as_str()).unwrap(),
            )?;
            db.update_incident_post(&tracked.post_fullname, cached.get_hash())?;
        }

        Ok(())
    }

    pub fn update_status(
        &mut self,
        db: &MlapiDb,
        reddit: &RouxClient,
        status: &StatusClient,
        cached: &mut CachedIncidentSubmissions,
        is_summary: bool,
        config: &SubredditStatusConfig,
    ) -> anyhow::Result<()> {
        let mut unseen = db.get_unresolved_incident_posts(self.lower.as_str())?;

        for incident in &cached.incidents {
            if let Some(tracked) =
                db.get_incident_post(self.lower.as_str(), incident.id.as_str())?
            {
                unseen.remove(&incident.id);
                if needs_update(incident, &tracked) {
                    let cached =
                        CachedIncidentSubmissions::get_submission(&mut cached.cache, incident)?;

                    self.update_incident_post(db, &tracked, cached, reddit)?;
                }
            } else if incident.impact >= config.min_impact {
                unseen.remove(&incident.id);

                let cached =
                    CachedIncidentSubmissions::get_submission(&mut cached.cache, incident)?;

                self.send_incident_post(
                    db,
                    incident.id.as_str(),
                    reddit,
                    config.flair_id.as_ref().map(|s| s.as_str()),
                    cached,
                )?;
            }
        }

        if !is_summary {
            // from a webhook, so it is expected that other incidents are missing
            return Ok(());
        }

        for unseen in unseen {
            let incident = status.get_incident(&unseen)?;

            CachedIncidentSubmissions::add(&mut cached.cache, &incident)?;
            let cached = CachedIncidentSubmissions::get_submission(&mut cached.cache, &incident)?;

            let Some(post) = db.get_incident_post(self.lower.as_str(), &incident.id)? else {
                continue;
            };

            self.update_incident_post(db, &post, cached, reddit)?;
            db.resolve_incident_post(&post.post_fullname)?;
        }

        Ok(())
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

fn needs_update(incident: &Incident, tracked: &IncidentPostLite) -> bool {
    match incident.updated_at {
        Some(updated_at) if updated_at > tracked.updated_at => true,
        _ => false,
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
