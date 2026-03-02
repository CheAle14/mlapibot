use std::{fmt::Write, path::PathBuf};

use mlapibot_common::config::GlobalSettings;
use mlapibot_reddit::{QuickStopError, RedditClient};
use mlapibot_webhook::{WebhookClient, create_generic_error_message};

#[derive(clap::Args)]
pub struct RedditArgs {
    /// A read-only directory where files such as the templates are stored
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,
    /// A read/write storage directory
    #[arg(long, short('d'))]
    scratch_dir: PathBuf,
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
    pub async fn run(self) -> anyhow::Result<()> {
        let Self {
            data_dir,
            scratch_dir,
            dry_run,
            status_webhook,
            admin,
            release,
        } = self;

        let settings = crate::get_global_settings(&scratch_dir)?;

        let analyzers = mlapibot_analysis::load_scams()?;

        let panic_webhook = settings.webhook_url.clone();

        let mut client = RedditClient::new(
            &analyzers,
            data_dir,
            dry_run,
            status_webhook,
            admin,
            settings,
            !release,
        )
        .await?;

        match client.run().await {
            Ok(()) => Ok(()),
            Err(error) => {
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
                        client.send(&message).await?;
                    }
                }

                if error.downcast_ref::<QuickStopError>().is_some() {
                    println!("Quickly stopped successfully");
                    return Ok(());
                }

                Err(error)
            }
        }
    }
}
