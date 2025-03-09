use clap::Parser;
use reddit::RedditArgs;
use single::SingleArgs;

mod reddit;
mod single;

#[derive(clap::Parser)]
pub struct MainArgs {
    #[clap(subcommand)]
    commands: MainCommands,
}

#[derive(clap::Subcommand)]
pub enum MainCommands {
    Reddit(RedditArgs),
    Test(SingleArgs),
}

fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();

    match args.commands {
        MainCommands::Reddit(reddit) => reddit.run(),
        MainCommands::Test(single) => single.run(),
    }
}
