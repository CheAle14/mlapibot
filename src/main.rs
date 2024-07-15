use std::{borrow::Cow, path::PathBuf};

use analysis::{get_best_analysis, load_scams, Analyzer};
use clap::{Args, CommandFactory, Parser, Subcommand};
use context::Context;
use reddit::RedditClient;
use serde::Deserialize;
use url::Url;

mod analysis;
mod context;
mod groups;
mod ocr;
mod reddit;
mod statics;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Analyzes a single image, printing the result
    Test(TestInfo),
    /// Runs the reddit bot using specified credentials
    Reddit(RedditInfo),
}

#[derive(Args)]
struct TestInfo {
    /// The input image path, which will be OCR-ed
    #[arg(short, long, group = "input")]
    file: Option<PathBuf>,
    /// The input image link, which will be downloaded and then OCR-ed
    #[arg(short, long, group = "input")]
    link: Option<Url>,
    /// Path where the seen words will be rendered to
    #[arg(short, long, default_value = "seen.png")]
    seen: PathBuf,
    /// Path where the trigger words will be rendered to
    #[arg(short, long, default_value = "trigger.png")]
    trigger: PathBuf,
}

#[derive(Clone, Args, Deserialize)]
#[group(required = false, multiple = true)]
struct RedditCredentials {
    #[arg(long)]
    client_id: String,
    #[arg(long)]
    client_secret: String,
    #[arg(long)]
    username: String,
    #[arg(long)]
    password: String,
}

#[derive(Args)]
struct RedditInfo {
    /// Path where files are to be stored
    #[arg(long, short)]
    data_dir: PathBuf,
    /// The subreddits whose posts are monitored
    #[arg(short, long)]
    subreddits: Vec<String>,
}

impl RedditInfo {
    pub fn get_credentials(&self) -> anyhow::Result<Cow<RedditCredentials>> {
        let credentials_file = self.data_dir.join("credentials.json");
        let mut file = std::fs::File::open(credentials_file)?;
        let parsed = serde_json::from_reader(&mut file)?;
        Ok(Cow::Owned(parsed))
    }
}

fn test_single(analyzers: &[Analyzer], args: &TestInfo) -> anyhow::Result<()> {
    let ctx = if args.file.is_some() {
        let file = args.file.as_ref().unwrap();
        Context::from_cli_path(file)?
    } else {
        let link = args.link.as_ref().unwrap();
        Context::from_cli_link(link)?
    };

    match get_best_analysis(&ctx, analyzers)? {
        Some((result, anal)) => {
            println!("{}:\r\n{:?}", anal.name, result.get_markdown(&ctx));

            for img in &ctx.images {
                let img = img.get_seen_words_image();
                img.save(&args.seen)?;
            }
            for img in result.get_trigger_images(&ctx)? {
                img.save(&args.trigger)?;
            }
        }
        None => {
            println!("Nothing detected");
        }
    }

    Ok(())
}

fn run_reddit(analyzers: &[Analyzer], args: &RedditInfo) -> anyhow::Result<()> {
    let mut client = RedditClient::new(analyzers, args)?;
    client.run()
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let analyzers = load_scams()?;

    match cli.command {
        SubCommand::Test(test) => test_single(&analyzers, &test),
        SubCommand::Reddit(reddit) => {
            if reddit.subreddits.len() == 0 {
                let mut cmd = Cli::command();
                cmd.error(
                    clap::error::ErrorKind::MissingRequiredArgument,
                    "at least one subreddit is required",
                )
                .exit();
            }
            run_reddit(&analyzers, &reddit)
        }
    }
}
