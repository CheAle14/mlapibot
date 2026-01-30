use std::{fmt::Write, path::PathBuf};

use anyhow::Context;
use mlapibot_reddit::{
    QuickStopError, RedditClient,
    config::{GlobalSettings, SubredditsConfig},
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
    pub fn get_global_settings(&self) -> anyhow::Result<GlobalSettings> {
        let settings_file = self.scratch_dir.join("settings.toml");

        let text = std::fs::read_to_string(&settings_file)
            .with_context(|| format!("reading {settings_file:?}"))?;

        let parsed = toml::from_str(&text)?;
        Ok(parsed)
    }

    pub fn get_subreddits_config(&self) -> anyhow::Result<SubredditsConfig> {
        let config = self.scratch_dir.join("subreddits.json");
        let mut file =
            std::fs::File::open(&config).with_context(|| format!("reading {config:?}"))?;

        let parsed = serde_json::from_reader(&mut file).context("subreddits.json config")?;
        Ok(parsed)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let settings = self.get_global_settings()?;
        let subreddits_config = self.get_subreddits_config()?;

        let Self {
            data_dir,
            scratch_dir,
            dry_run,
            status_webhook,
            admin,
            release,
        } = self;

        let analyzers = mlapibot_analysis::load_scams()?;

        let panic_webhook = settings.webhook_url.clone();

        let mut client = RedditClient::new(
            &analyzers,
            data_dir,
            scratch_dir.join("database.db"),
            dry_run,
            status_webhook,
            admin,
            settings,
            subreddits_config,
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
