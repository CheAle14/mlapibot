use std::path::PathBuf;

use mlapibot_analysis::{Context, Url};

#[derive(clap::Args)]
pub struct SingleArgs {
    /// Path where the seen words will be rendered to
    #[arg(short, long, default_value = "seen.png")]
    seen: PathBuf,
    /// Path where the trigger words will be rendered to
    #[arg(short, long, default_value = "trigger.png")]
    trigger: PathBuf,
    // /// A particular analyzer to use, or all of them if absent.
    // #[arg(short, long)]
    // analzyer: Option<String>,
    /// Whether to display the markdown formatted template string as well
    #[arg(short, long)]
    markdown: bool,

    #[arg(long)]
    debug: bool,

    /// The https?:// link, or a path to file.
    link_or_file: String,
}

impl SingleArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let mut warnings = Vec::new();
        let mut ctx = if self.link_or_file.starts_with("http") {
            Context::new(
                std::iter::once(Url::parse(&self.link_or_file).expect("is url")),
                None,
                None,
                &mut warnings,
            )
            .await?
        } else {
            Context::new_path(PathBuf::from(&self.link_or_file))?
        };

        for warning in warnings {
            eprintln!("{warning}");
        }

        ctx.debug = self.debug;

        println!(
            "Saw words:\r\n{}",
            ctx.images.first().unwrap().words().join(" ")
        );

        // let analyzers = mlapibot_analysis::load_scams()?;
        // if let Some(name) = &self.analzyer {
        //     let analyzer = analyzers
        //         .iter()
        //         .find(|a| &a.name == name)
        //         .expect("analzyer exists by that name");

        //     std::fs::write("analyzer.txt", format!("{analyzer:#?}")).unwrap();

        //     match analyzer.analyze(&ctx)? {
        //         Some(detect) => {
        //             println!("{name} saw: {:?}", detect.get_markdown(&ctx)?)
        //         }
        //         None => {
        //             println!("{name} detected nothing");
        //             if analyzer.disabled {
        //                 println!("because it was disabled!");
        //             }
        //         }
        //     }
        // } else {
        //     match get_best_analysis(&ctx, &analyzers)? {
        //         Some((result, anal)) => {
        //             println!("{}:\r\n{:?}", anal.name, result.get_markdown(&ctx));

        //             for img in &ctx.images {
        //                 let img = img.get_seen_words_image();
        //                 img.save(&self.seen)?;
        //             }
        //             for img in result.get_trigger_images(&ctx)? {
        //                 img.save(&self.trigger)?;
        //             }

        //             if self.markdown {
        //                 let templates = tera::Tera::new("./data/templates/*.md").unwrap();

        //                 let template = match anal.template.name() {
        //                     Some(text) => templates.render(text, &tera::Context::new()).unwrap(),
        //                     None => String::from("<analyzer has no template>"),
        //                 };

        //                 println!("\r\n{template}");
        //             }
        //         }
        //         None => {
        //             println!("Nothing detected");
        //         }
        //     }
        // }

        Ok(())
    }
}
