use clap::Parser;

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
    Reddit(reddit::RedditArgs),
    Test(single::SingleArgs),
    Download(download::DownloadArgs),
}

fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();

    match args.commands {
        MainCommands::Reddit(reddit) => reddit.run(),
        MainCommands::Test(single) => single.run(),
        MainCommands::Download(download) => download.run(),
    }
}
