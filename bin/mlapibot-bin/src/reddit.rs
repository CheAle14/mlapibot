use std::{any::Any, fmt::Write, path::PathBuf};

use anyhow::Context;
use mlapibot_common::LowercaseString;
use mlapibot_reddit::{
    RedditClient,
    config::{RedditCredentials, SubredditsConfig},
};
use mlapibot_webhook::{WebhookClient, create_generic_error_message};

#[derive(clap::Args)]
pub struct RedditArgs {
    /// A read-only directory where files such as the templates are stored
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,
    /// A read/write storage directory
    #[arg(long, short('d'))]
    scratch_dir: PathBuf,
    /// The subreddits whose posts are monitored
    #[arg(short, long)]
    subreddits: Vec<LowercaseString>,
    #[arg(long, default_value = "false")]
    dry_run: bool,
    /// Whether we are running in production or not
    #[arg(long)]
    release: bool,
    /// If present, bind a HTTP listener to the provided address to listen for status webhooks.
    #[arg(long)]
    status_webhook: Option<String>,
    #[arg(long)]
    admin: Option<String>,
}

impl RedditArgs {
    pub fn get_credentials(&self) -> anyhow::Result<RedditCredentials> {
        let credentials_file = self.scratch_dir.join("credentials.json");
        let mut file = std::fs::File::open(&credentials_file)
            .with_context(|| format!("reading {credentials_file:?}"))?;

        let parsed = serde_json::from_reader(&mut file)?;
        Ok(parsed)
    }

    pub fn get_subreddits_config(&self) -> anyhow::Result<SubredditsConfig> {
        let config = self.scratch_dir.join("subreddits.json");
        let mut file =
            std::fs::File::open(&config).with_context(|| format!("reading {config:?}"))?;

        let parsed = serde_json::from_reader(&mut file)?;
        Ok(parsed)
    }

    pub fn run(self) -> anyhow::Result<()> {
        let credentials = self.get_credentials()?;
        let subreddits_config = self.get_subreddits_config()?;

        let Self {
            data_dir,
            scratch_dir,
            subreddits,
            dry_run,
            status_webhook,
            admin,
            release,
        } = self;

        let analyzers = mlapibot_analysis::load_scams()?;

        let panic_webhook = credentials.webhook_url.clone();

        let result = std::panic::catch_unwind(|| {
            let mut client = RedditClient::new(
                &analyzers,
                data_dir,
                scratch_dir.join("database.db"),
                subreddits,
                dry_run,
                status_webhook,
                admin,
                credentials,
                subreddits_config,
                !release,
            )?;

            client.run()
        });

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                if release {
                    if let Some(webhook) = panic_webhook {
                        let mut client = WebhookClient::new(webhook)?;

                        let mut s = String::with_capacity(512);

                        let _ = writeln!(s, "Error: {error}");

                        if let Some(source) = error.source() {
                            let _ = writeln!(s, "\nCaused by:");

                            for (idx, next) in
                                std::iter::successors(Some(source), |err| err.source()).enumerate()
                            {
                                let _ = writeln!(s, "  {idx}: {next}");
                            }
                        }

                        let message = create_generic_error_message("Fatal error occured", s);
                        client.send(&message)?;
                    }
                }

                Err(error)
            }
            Err(panic) if release => {
                if let Some(webhook) = panic_webhook {
                    let mut client = WebhookClient::new(webhook)?;

                    let msg = match downcast_panic_str(&panic) {
                        Some(d) => d,
                        None => "Unable to retrieve panic message",
                    };

                    let message = create_generic_error_message("Fatal panic occured", msg);
                    client.send(&message)?;
                }

                std::panic::resume_unwind(panic)
            }
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

fn downcast_panic_str(panic: &Box<dyn Any + Send>) -> Option<&str> {
    if let Some(v) = panic.downcast_ref::<&str>() {
        return Some(*v);
    }

    if let Some(v) = panic.downcast_ref::<String>() {
        return Some(v.as_str());
    }

    return None;
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use super::downcast_panic_str;

    #[test]
    pub fn downcasts_str() {
        let panic = catch_unwind(|| panic!("hello world")).unwrap_err();
        let text = downcast_panic_str(&panic);
        assert_eq!(text, Some("hello world"));
    }

    #[test]
    pub fn downcasts_string() {
        let panic = catch_unwind(|| panic!("hello {} world", 5)).unwrap_err();
        let text = downcast_panic_str(&panic);
        assert_eq!(text, Some("hello 5 world"));
    }
}
