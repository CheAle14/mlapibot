#[derive(clap::Subcommand)]
pub enum DbCommands {
    /// Removes the provided post from the Monitored table.
    Unmonitor { fullname: String },
}

impl DbCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        let db = mlapibot_datastore::MlapiDb::new("database.db")?;

        match self {
            DbCommands::Unmonitor { fullname } => {
                db.delete_monitored(&fullname)?;
                println!("Deleted {fullname}");
            }
        }

        Ok(())
    }
}
