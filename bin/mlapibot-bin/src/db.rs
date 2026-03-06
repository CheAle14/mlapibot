use std::path::PathBuf;

use mlapibot_database_v2::repos::monitor::MonitorRepo;

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

    /// Applies any outstanding migrations
    Migrate,

    /// Removes the last `count` migrations.
    Unmigrate {
        #[clap(default_value_t = 1)]
        count: usize,
    },
}

impl DbArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let settings = crate::get_global_settings(&self.scratch_dir)?;
        let mut db = mlapibot_database_v2::client::PgClientBuilder::new(&settings.database_uri)
            .connect()
            .await?;

        match self.cmd {
            DbCommands::Unmonitor { fullname } => {
                if db.delete_monitored(&fullname).await? {
                    println!("Deleted {fullname}");
                } else {
                    println!("Hmm, {fullname:?} was not monitored?");
                }
            }

            DbCommands::Migrate => {
                // the client auto-migrates after connecting.
                println!("Done!");
            }

            DbCommands::Unmigrate { count } => {
                mlapibot_database_v2::migrations::drop_migrations(&mut db, count).await?
            }
        }

        Ok(())
    }
}
