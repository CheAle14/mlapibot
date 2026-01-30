use clap::Parser;

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
    #[command(subcommand)]
    Db(db::DbCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();

    match args.commands {
        MainCommands::Reddit(reddit) => reddit.run().await,
        MainCommands::Test(single) => single.run().await,
        MainCommands::Download(download) => download.run().await,
        MainCommands::Db(db) => db.run().await,
    }
}
