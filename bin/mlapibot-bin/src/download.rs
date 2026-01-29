use std::path::PathBuf;

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

        let downloaded = mlapibot_analysis::download_file(&url)
            .await?
            .expect("can download");

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

        downloaded.move_and_keep(&dir)?;

        println!("File downloaded to {dir:?}");

        Ok(())
    }
}
