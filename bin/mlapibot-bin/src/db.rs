use std::path::PathBuf;

use mlapibot_database_v2::{migrations::apply_migrations, repos::monitor::MonitorRepo};

#[derive(clap::Args)]
pub struct DbArgs {
    #[clap(long, short('d'))]
    scratch_dir: PathBuf,

    #[clap(subcommand)]
    cmd: DbCommands,
}

#[derive(clap::Subcommand)]
enum DbCommands {
    /// Removes the provided post from the Monitored table.
    Unmonitor { fullname: String },

    /// Runs all migrations
    Migrate,
}

impl DbArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let settings = crate::get_global_settings(&self.scratch_dir)?;
        let mut db =
            mlapibot_database_v2::client::PgClient::connect(&settings.database_uri).await?;

        match self.cmd {
            DbCommands::Unmonitor { fullname } => {
                if db.delete_monitored(&fullname).await? {
                    println!("Deleted {fullname}");
                } else {
                    println!("Hmm, {fullname:?} was not monitored?");
                }
            }
            DbCommands::Migrate => {
                apply_migrations(&mut db).await?;
                println!("Done!");
            }
        }

        Ok(())
    }
}
