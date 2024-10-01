use std::{collections::HashMap, io, path::PathBuf, sync::mpsc::Sender};

use chrono::{DateTime, Utc};
use roux::{api::ThingFullname, builders::submission::SubmissionSubmitBuilder};
use serde::{Deserialize, Serialize};
use statuspage::incident::{Incident, IncidentStatus};

use crate::utils::clamp;

use super::{subreddit::RouxSubreddit, RouxClient};

#[derive(Serialize, Deserialize)]
pub struct StatusSubmission {
    post_id: ThingFullname,
    last_updated: DateTime<Utc>,
    #[serde(default)]
    removal_count: usize,
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

    // We want the updates to appear from newest to oldest (top to bottom, respectively),
    // but the status title needs to be determined from oldest to newest.
    // So look oldest to newest first and push to a temporary buffer
    // then reverse when writing it back to the actual text.
    let mut texts = Vec::new();
    let mut last_status = IncidentStatus::Postmortem;

    for update in incident.incident_updates.iter().rev() {
        let pdt = update.created_at.with_timezone(&pdt);

        let mut text = String::new();
        write!(text, "### ")?;

        if update.status == last_status {
            write!(text, "Update")?;
        } else {
            write!(text, "{:?}", update.status)?;
            last_status = update.status;
        }

        writeln!(
            text,
            "  \r\n{}  \r\n\r\n{}\r\n\r\n---\r\n\r\n",
            update.body,
            pdt.format("%b %e, %Y - %H:%M PDT")
        )?;

        texts.push(text);
    }

    for t in texts.into_iter().rev() {
        text.push_str(&t);
    }

    if incident.components.len() > 0 {
        writeln!(text, "This issue affects:  \r\n")?;
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
        incident_last_updated: DateTime<Utc>,
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
                last_updated: incident_last_updated,
                removal_count: 0,
            },
        );

        self.save()
    }

    pub fn potentially_remove(&mut self, incident_id: &str) -> anyhow::Result<()> {
        let do_remove = if let Some(thing) = self.map.posts.get_mut(incident_id) {
            thing.removal_count += 1;
            if thing.removal_count > 60 {
                true
            } else {
                self.save()?;
                false
            }
        } else {
            false
        };

        if do_remove && self.map.posts.remove(incident_id).is_some() {
            self.save()?;
        }

        Ok(())
    }

    pub fn update(
        &mut self,
        reddit: &RouxClient,
        incident_id: &str,
        incident_last_updated: DateTime<Utc>,
        text: &str,
    ) -> anyhow::Result<()> {
        if let Some(state) = self.map.posts.get_mut(incident_id) {
            reddit.edit(text, &state.post_id)?;
            state.last_updated = incident_last_updated;
        }
        Ok(())
    }
}

pub struct CachedIncidentSubmissions {
    pub incidents: Vec<Incident>,
    pub cache: HashMap<String, SubmissionSubmitBuilder>,
}

impl CachedIncidentSubmissions {
    pub fn new(incidents: Vec<Incident>) -> Self {
        Self {
            incidents,
            cache: HashMap::new(),
        }
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

pub enum WebhookEvent {
    IncidentUpdate(Box<Incident>),
    OtherUpdate,
    Closed,
}

pub fn start_webhook_listener_thread(channel: Sender<WebhookEvent>, addr: &str) {
    let addr = addr.to_owned();

    std::thread::spawn(move || {
        let chnl = channel.clone();
        rouille::Server::new(addr, move |request| {
            println!("[status-webhook] {} {}", request.method(), request.url());
            let Some(body) = request.data() else {
                return rouille::Response::empty_404();
            };

            let parsed: statuspage::webhook::StatusWebhook = match serde_json::from_reader(body) {
                Ok(value) => value,
                Err(err) => {
                    println!("[status-webhook] {err:?}");
                    // *something* has happened, so trigger a refresh anyway
                    chnl.send(WebhookEvent::OtherUpdate).unwrap();
                    return rouille::Response::text("failed to parse json").with_status_code(500);
                }
            };

            let event = match parsed.payload {
                statuspage::webhook::WebhookPayload::Incident { incident } => {
                    WebhookEvent::IncidentUpdate(Box::new(incident))
                }
                _ => WebhookEvent::OtherUpdate,
            };

            chnl.send(event).unwrap();

            rouille::Response::empty_204()
        })
        .expect("Failed to start server")
        .run();
        channel.send(WebhookEvent::Closed).unwrap();
    });
}
