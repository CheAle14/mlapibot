use std::{collections::HashMap, sync::mpsc::Sender};

use statuspage::{
    component::Component,
    incident::{Incident, IncidentStatus},
};

use crate::utils::clamp;

use super::cached_submission::CachedSubmission;

pub fn get_title(incident: &Incident) -> anyhow::Result<String> {
    Ok(format!(
        "{:?} status issue: {}",
        incident.impact,
        clamp(&incident.name, 256)
    ))
}

pub fn get_markdown(
    incident: &Incident,
    all_components: &HashMap<String, Component>,
) -> anyhow::Result<String> {
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

        let mut grouped: HashMap<&String, Vec<&Component>> = HashMap::new();

        for component in &incident.components {
            if let Some(id) = &component.group_id {
                match grouped.get_mut(id) {
                    Some(vec) => vec.push(component),
                    None => {
                        grouped.insert(id, vec![component]);
                    }
                }
            } else {
                write!(text, "- **{}**", component.name)?;
                if let Some(desc) = &component.description {
                    writeln!(text, ": {desc}")?;
                } else {
                    writeln!(text, "  ")?;
                }
            }
        }

        for (group_id, components) in grouped {
            let (name, description) = match all_components.get(group_id) {
                Some(group) => (
                    group.name.as_str(),
                    group.description.as_ref().map(|s| s.as_str()),
                ),
                None => (group_id.as_str(), Some("(unknown group ID)")),
            };

            write!(text, "- **{name}** (")?;
            let mut first = true;
            for component in components {
                if !first {
                    write!(text, ", ")?;
                }
                first = false;
                write!(text, "{}", component.name)?;
            }
            write!(text, ")")?;

            if let Some(desc) = description {
                writeln!(text, ": {desc}")?;
            } else {
                writeln!(text, "  ")?;
            }
        }
    }

    Ok(text)
}
pub struct CachedIncidentSubmissions {
    pub incidents: Vec<Incident>,
    pub cache: HashMap<String, CachedSubmission>,
}

impl CachedIncidentSubmissions {
    pub fn new(incidents: Vec<Incident>) -> Self {
        Self {
            incidents,
            cache: HashMap::new(),
        }
    }

    pub fn add(
        this: &mut HashMap<String, CachedSubmission>,
        incident: &Incident,
        all_components: &HashMap<String, Component>,
    ) -> anyhow::Result<()> {
        let title = get_title(incident)?;
        let body = get_markdown(incident, all_components)?;
        this.insert(incident.id.clone(), CachedSubmission::new(title, body));
        Ok(())
    }

    pub fn get_submission<'a>(
        this: &'a mut HashMap<String, CachedSubmission>,
        incident: &Incident,
        all_components: &HashMap<String, Component>,
    ) -> anyhow::Result<&'a CachedSubmission> {
        if !this.contains_key(&incident.id) {
            Self::add(this, incident, all_components)?;
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
