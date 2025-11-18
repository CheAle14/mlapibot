use std::{collections::HashMap, sync::mpsc::Sender};

use chrono::{DateTime, FixedOffset, Utc};
use mlapibot_datastore::live_incident_posts::LiveIncidentPost;
use roux::builders::submission::SubmissionSubmitBuilder;
use statuspage::{
    component::Component,
    incident::{AffectedComponent, Incident, IncidentStatus, IncidentUpdate},
};

use crate::utils::clamp;

pub struct IncidentWithLive<'a> {
    pub incident: &'a Incident,
    pub live_thread: LiveIncidentPost,
}

impl<'a> IncidentWithLive<'a> {
    pub fn to_builder(&self) -> SubmissionSubmitBuilder {
        SubmissionSubmitBuilder::link(
            get_title(self.incident, 256).expect("String write should be infalliable"),
            format!("https://www.reddit.com/live/{}/", self.live_thread.fullname),
            false,
        )
        .with_send_replies(false)
    }
}

pub fn get_title(incident: &Incident, max_length: usize) -> anyhow::Result<String> {
    let prefix = match incident.impact {
        statuspage::incident::IncidentImpact::None => "Status issue",
        statuspage::incident::IncidentImpact::Maintenance => "Maintenance",
        statuspage::incident::IncidentImpact::Minor => "Minor status issue",
        statuspage::incident::IncidentImpact::Major => "Major status issue",
        statuspage::incident::IncidentImpact::Critical => "Critical status issue",
    };

    Ok(format!(
        "{prefix}: {}",
        clamp(&incident.name, max_length - (prefix.len() + ": ".len()))
    ))
}

pub fn write_affected_components_list(
    text: &mut String,
    incident: &Incident,
    all_components: &HashMap<String, Component>,
) -> anyhow::Result<()> {
    use std::fmt::Write;

    if incident.components.len() == 0 {
        return Ok(());
    }

    writeln!(text, "\r\n\r\n---\r\n\r\nThis issue affects:  \r\n")?;

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

    Ok(())
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
