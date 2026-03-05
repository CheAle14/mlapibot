use std::collections::HashMap;

use mlapibot_database_v2::repos::incidents::StatusIncident;
use roux::builders::submission::SubmissionSubmitBuilder;
use statuspage::{component::Component, incident::Incident};

use crate::utils::{BoO, clamp};

pub struct IncidentWithLive<'a> {
    pub incident: BoO<'a, Incident>,
    pub live_thread: StatusIncident,
}

impl<'a> IncidentWithLive<'a> {
    pub fn to_builder(&self) -> SubmissionSubmitBuilder {
        SubmissionSubmitBuilder::link(
            get_title(&self.incident, 256).expect("String write should be infalliable"),
            format!(
                "https://www.reddit.com/live/{}/",
                self.live_thread.live_fullname
            ),
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
