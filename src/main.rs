use std::path::PathBuf;

use analysis::{get_best_analysis, load_scams, Analyzer};
use clap::{Args, Parser, Subcommand};
use context::Context;
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

#[derive(Args)]
struct RedditInfo {
    #[arg(long)]
    client_id: String,
    #[arg(long)]
    client_secret: String,
    #[arg(long)]
    username: String,
    #[arg(long)]
    password: String,
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
    const USER_AGENT: &str = "mlapibot-rs v2.0.0 (by /u/DarkOverLordCO)";
    todo!();
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let analyzers = load_scams()?;

    match cli.command {
        SubCommand::Test(test) => test_single(&analyzers, &test),
        SubCommand::Reddit(reddit) => run_reddit(&analyzers, &reddit),
    }
}
