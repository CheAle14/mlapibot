use std::path::Path;

use anyhow::Context;
use clap::Parser;
use mlapibot_common::config::GlobalSettings;

mod db;
mod download;
mod reddit;
mod single;

#[derive(clap::Parser)]
pub struct MainArgs {
    #[clap(subcommand)]
    commands: MainCommands,
}

#[derive(clap::Subcommand)]
pub enum MainCommands {
    /// Run the main mlapibot reddit system
    Reddit(reddit::RedditArgs),
    /// Perform OCR and analysis on a single image
    Test(single::SingleArgs),
    /// Download the image to the tests folder
    Download(download::DownloadArgs),
    /// Perform manual operations against the database
    Db(db::DbArgs),
}

pub(crate) fn get_global_settings(scratch_dir: &Path) -> anyhow::Result<GlobalSettings> {
    let settings_file = scratch_dir.join("settings.toml");

    let text = std::fs::read_to_string(&settings_file)
        .with_context(|| format!("reading {settings_file:?}"))?;

    let parsed = toml::from_str(&text)?;
    Ok(parsed)
}

fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();

    if let MainCommands::Reddit(..) = &args.commands {
        // This needs to be ran before any threads are started, which means
        // before tokio's thread loop.
        systemd_socket::init().context("initializing systemd sockets")?;
    }

    async_start(args)
}

#[tokio::main]
async fn async_start(args: MainArgs) -> anyhow::Result<()> {
    match args.commands {
        MainCommands::Reddit(reddit) => reddit.run().await,
        MainCommands::Test(single) => single.run().await,
        MainCommands::Download(download) => download.run().await,
        MainCommands::Db(db) => db.run().await,
    }
}
