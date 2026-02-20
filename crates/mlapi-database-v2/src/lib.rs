pub mod client;
pub mod errors;
pub mod migrations;
pub mod repos;

#[cfg(test)]
mod tests {
    use crate::{client::PgClient, errors::DbResult, migrations::apply_migrations};

    #[tokio::test]
    pub async fn test_db() -> DbResult<()> {
        let mut db = PgClient::connect("postgres://postgres:postgres@localhost/mlapibot").await?;
        apply_migrations(&mut db).await?;

        Ok(())
    }
}
