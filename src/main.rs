use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use analysis::{get_best_analysis, load_scams, Analyzer};
use clap::{Args, CommandFactory, Parser, Subcommand};
use context::Context;
use reddit::{config::SubredditsConfig, RedditClient};
use serde::Deserialize;
use statuspage::incident::IncidentImpact;
use utils::LowercaseString;

mod analysis;
mod context;
mod error;
mod groups;
mod imgur;
mod ocr;
mod reddit;
mod statics;
pub(crate) mod url;
pub(crate) mod utils;
mod webhook;

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
    /// Analyzes all images in the sub-folders, expecting them to be detected by the analyzer whose name
    /// is equal to the image folder name. If that folder is `none`, then expects no detection from any.
    TestRun(TestRunInfo),
}

#[derive(Args)]
struct TestRunInfo {
    #[arg(long, default_value = "./tests")]
    dir: PathBuf,
}

#[derive(Args)]
struct TestInfo {
    /// The input image path, which will be OCR-ed
    #[arg(short, long, group = "input")]
    file: Option<PathBuf>,
    /// The input image link, which will be downloaded and then OCR-ed
    #[arg(short, long, group = "input")]
    link: Option<url::Url>,
    /// Path where the seen words will be rendered to
    #[arg(short, long, default_value = "seen.png")]
    seen: PathBuf,
    /// Path where the trigger words will be rendered to
    #[arg(short, long, default_value = "trigger.png")]
    trigger: PathBuf,
    /// A particular analyzer to use, or all of them if absent.
    #[arg(short, long)]
    analzyer: Option<String>,
    /// Whether to display the markdown formatted template string as well
    #[arg(short, long)]
    markdown: bool,
}

#[derive(Clone, Deserialize)]
pub struct ImgurCredentials {
    imgur_client_id: String,
    imgur_client_secret: String,
}

#[derive(Clone, Deserialize)]
struct RedditCredentials {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    webhook_url: Option<String>,
    #[serde(flatten)]
    imgur_credentials: Option<ImgurCredentials>,
}

#[derive(Args)]
struct RedditInfo {
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
    /// If present, bind a HTTP listener to the provided address to listen for status webhooks.
    #[arg(long)]
    status_webhook: Option<String>,
    #[arg(long)]
    admin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubredditStatusConfig {
    pub min_impact: IncidentImpact,
    pub flair_id: Option<String>,
}

impl RedditInfo {
    pub fn get_credentials(&self) -> anyhow::Result<Cow<RedditCredentials>> {
        let credentials_file = self.scratch_dir.join("credentials.json");
        let mut file = std::fs::File::open(credentials_file)?;
        let parsed = serde_json::from_reader(&mut file)?;
        Ok(Cow::Owned(parsed))
    }

    pub fn get_subreddits_config(&self) -> anyhow::Result<SubredditsConfig> {
        let config = self.scratch_dir.join("subreddits.json");
        let mut file = std::fs::File::open(config)?;
        let parsed = serde_json::from_reader(&mut file)?;
        Ok(parsed)
    }
}

fn test_single(analyzers: &[Analyzer], args: &TestInfo) -> anyhow::Result<()> {
    let (mut ctx, warnings) = if args.file.is_some() {
        let file = args.file.as_ref().unwrap();
        tryw!(Context::from_cli_path(file), Result::Err)
    } else {
        let link = args.link.as_ref().unwrap();
        tryw!(Context::from_cli_link(link), Result::Err)
    };

    for warning in warnings {
        eprintln!("Warning: {warning:?}");
    }

    ctx.debug = args.analzyer.is_some();

    println!(
        "Saw words:\r\n{}",
        ctx.images.first().unwrap().words().join(" ")
    );

    if let Some(name) = &args.analzyer {
        let analyzer = analyzers
            .iter()
            .find(|a| &a.name == name)
            .expect("analzyer exists by that name");
        match analyzer.analyze(&ctx)? {
            Some(detect) => {
                println!("{name} saw: {:?}", detect.get_markdown(&ctx)?)
            }
            None => {
                println!("{name} detected nothing");
            }
        }
    } else {
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
    }

    Ok(())
}

fn run_reddit(analyzers: &[Analyzer], args: &RedditInfo) -> anyhow::Result<()> {
    let mut client = RedditClient::new(analyzers, args)?;
    match client.run() {
        r @ Ok(()) => r,
        Err(err) => {
            // we might fail at sending the webhook, so make sure we log the underlying error
            let message = crate::webhook::create_generic_error_message(
                "A fatal error has occured in mlapibot!",
                &err,
            );
            client.send_webhook(&message)?;
            Err(err)
        }
    }
}

fn test_folder(analyzers: &[Analyzer], args: TestRunInfo) -> anyhow::Result<()> {
    for dir in std::fs::read_dir(args.dir)? {
        let dir = dir?;

        println!("Testing {:?}", dir.path());
        let is_none = dir.file_name().eq_ignore_ascii_case("none");

        for file in std::fs::read_dir(dir.path())? {
            let file = file?;

            let (ctx, warnings) = tryw!(Context::from_cli_path(file.path()), Result::Err);

            for warning in warnings {
                eprintln!("  Warning: {warning:?}");
            }

            if is_none {
                for yzer in analyzers {
                    if let Some(det) = yzer.analyze(&ctx)? {
                        panic!("  failed {file:?} not none -> {det:?}");
                    }
                }
                println!("  {file:?} passed");
            } else {
                let mut found = false;
                for yzer in analyzers {
                    if dir.file_name().eq_ignore_ascii_case(&yzer.name) {
                        found = true;
                        match yzer.analyze(&ctx)? {
                            None => panic!("  failed {file:?} not detected"),
                            Some(_) => {
                                println!("  {file:?} passed");
                                break;
                            }
                        }
                    }
                }
                if !found {
                    panic!(
                        "  failed {file:?} no analyzer with the name {:?}",
                        dir.file_name()
                    );
                }
            }
        }

        println!()
    }

    Ok(())
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
        SubCommand::TestRun(args) => test_folder(&analyzers, args),
    }
}
