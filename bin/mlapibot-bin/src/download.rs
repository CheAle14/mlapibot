use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use mlapibot_analysis::Url;

#[derive(clap::Args)]
pub struct DownloadArgs {
    /// The test folder to download it to. If not provided, the current working directory.
    #[arg(short, long)]
    test: Option<String>,

    /// The https?:// link to the image.
    link: String,
}

impl DownloadArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let url = Url::parse(&self.link)
            .expect("input is a valid URL")
            .fix()
            .expect("is https");

        let client = reqwest::Client::default();
        let downloaded = mlapibot_analysis::download_file(&client, &url).await?;

        let mut dir = match &self.test {
            Some(dir) => {
                let mut path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
                path.pop();
                path.pop();
                path.push("tests");
                path.push(dir);

                path
            }
            None => std::env::current_dir()?,
        };

        dir.push(
            url.filename()
                .expect("has filename since we downloaded it before"),
        );

        let (mut file, path) = downloaded.keep().context("make file permanent")?;
        file.flush().context("flush file")?;

        move_file(&path, &dir).with_context(|| format!("move {path:?} to {dir:?}"))?;

        println!("File downloaded to {dir:?}");

        Ok(())
    }
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            std::fs::copy(from, to)?;
            std::fs::remove_file(from)
        }
        Err(err) => {
            eprintln!("failed {:?}", err.kind());
            Err(err)
        }
    }
}
