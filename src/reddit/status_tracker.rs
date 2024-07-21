use std::{collections::HashMap, io, path::PathBuf};

use chrono::{DateTime, Utc};
use roux::{api::ThingId, builders::submission::SubmissionSubmitBuilder};
use serde::{Deserialize, Serialize};
use statuspage::{incident::Incident, summary::Summary};

use crate::utils::clamp;

use super::{subreddit::RouxSubreddit, RouxClient};

#[derive(Serialize, Deserialize)]
pub struct StatusSubmission {
    post_id: ThingId,
    last_updated: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct StatusMap {
    pub posts: HashMap<String, StatusSubmission>,
}

pub struct StatusTracker {
    pub map: StatusMap,
    path: PathBuf,
}

pub fn get_title(incident: &Incident) -> anyhow::Result<String> {
    Ok(format!(
        "{:?} status issue: {}",
        incident.impact,
        clamp(&incident.name, 256)
    ))
}

pub fn get_markdown(incident: &Incident) -> anyhow::Result<String> {
    use std::fmt::Write;

    let mut text = String::new();
    writeln!(text, "## [{}]({})\r\n", incident.name, incident.shortlink)?;

    let pdt = chrono_tz::PST8PDT;

    for update in &incident.incident_updates {
        let pdt = update.created_at.with_timezone(&pdt);
        writeln!(
            text,
            "### {:?}  \r\n{}  \r\n\r\n{}\r\n\r\n---\r\n\r\n",
            update.status,
            update.body,
            pdt.format("%b %e, %Y - %H:%M PDT")
        )?
    }

    if incident.components.len() > 0 {
        writeln!(text, "This issue affects:  ")?;
        for component in &incident.components {
            write!(text, "- **{}**", component.name)?;
            if let Some(desc) = &component.description {
                writeln!(text, ": {desc}")?;
            } else {
                writeln!(text, "  ")?;
            }
        }
    }

    Ok(text)
}

impl StatusTracker {
    pub fn new(path: PathBuf) -> Self {
        let map = match std::fs::File::open(&path) {
            Ok(file) => serde_json::from_reader(file).expect("can parse status file as json"),
            Err(e) if e.kind() == io::ErrorKind::NotFound => StatusMap {
                posts: HashMap::new(),
            },
            Err(e) => panic!("Failed to open {path:?}: {e:?}"),
        };

        Self { map, path }
    }

    pub fn is_tracking(&self, incident_id: &str) -> bool {
        self.map.posts.contains_key(incident_id)
    }

    pub fn needs_update(&self, incident: &Incident) -> bool {
        if incident.updated_at.is_none() {
            // clearly no updates
            return false;
        }
        if let Some(inc) = self.map.posts.get(&incident.id) {
            inc.last_updated < incident.updated_at.unwrap()
        } else {
            false
        }
    }

    fn save(&self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer(file, &self.map)?;
        Ok(())
    }

    pub fn add(
        &mut self,
        incident_id: &str,
        reddit: &RouxClient,
        subreddit: &RouxSubreddit,
        submission: &SubmissionSubmitBuilder,
    ) -> anyhow::Result<()> {
        println!("Sending incident to /r/{}", subreddit.name);
        let submission = reddit.submit(&subreddit.name, submission)?;
        println!("Incident posted as {:?}", submission.name());

        self.map.posts.insert(
            incident_id.to_owned(),
            StatusSubmission {
                post_id: submission.name().clone(),
                last_updated: chrono::Utc::now(),
            },
        );

        self.save()
    }

    pub fn remove(&mut self, incident_id: &str) -> anyhow::Result<()> {
        let any = self.map.posts.remove(incident_id).is_some();
        if any {
            self.save()?;
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        reddit: &RouxClient,
        incident_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        if let Some(state) = self.map.posts.get_mut(incident_id) {
            reddit.edit(text, &state.post_id)?;
            state.last_updated = Utc::now();
        }
        Ok(())
    }
}

pub struct CachedSummary {
    pub summary: Summary,
    pub cache: HashMap<String, SubmissionSubmitBuilder>,
}

impl CachedSummary {
    pub fn new(summary: Summary) -> anyhow::Result<Self> {
        Ok(Self {
            summary,
            cache: HashMap::new(),
        })
    }

    pub fn add(
        this: &mut HashMap<String, SubmissionSubmitBuilder>,
        incident: &Incident,
    ) -> anyhow::Result<()> {
        let title = get_title(incident)?;
        let body = get_markdown(incident)?;
        this.insert(
            incident.id.clone(),
            SubmissionSubmitBuilder::text(title, body),
        );
        Ok(())
    }

    pub fn get_submission<'a>(
        this: &'a mut HashMap<String, SubmissionSubmitBuilder>,
        incident: &Incident,
    ) -> anyhow::Result<&'a SubmissionSubmitBuilder> {
        if !this.contains_key(&incident.id) {
            Self::add(this, incident)?;
        }
        Ok(this.get(&incident.id).unwrap())
    }
}
