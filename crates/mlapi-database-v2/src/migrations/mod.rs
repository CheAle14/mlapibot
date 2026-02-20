use tokio_postgres::Transaction;

use crate::{client::PgClient, errors::DbResult};

pub trait Migration {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }

    async fn apply(conn: &Transaction<'_>) -> DbResult<()>;
    async fn undo(conn: &Transaction<'_>) -> DbResult<()>;
}

macro_rules! define_migrations {
    [$($module:ident::$struct:ident),* $(,)?] => {
        $(
            mod $module;
        )*

        pub async fn apply_migrations(client: &mut crate::client::PgClient) -> DbResult<()> {
            let migrations = match get_done_migrations(&client).await? {
                Some(m) => m,
                None => {
                    println!("[db] No migrations table, creating");
                    create_migration_table(&client).await?;
                    Vec::new()
                }
            };

            $(
              let name = $module::$struct::name();
              if !migrations.iter().any(|i| i == name) {
                  apply_migration_in_transaction::<$module::$struct>(client).await?;
              }
            )*

            Ok(())
        }
    };
}

async fn create_migration_table(client: &PgClient) -> DbResult<()> {
    client
        .execute(
            r#"CREATE TABLE _migrations (
            name        TEXT            PRIMARY KEY,
            added_at    TIMESTAMPTZ     DEFAULT CURRENT_TIMESTAMP
        );"#,
            &[],
        )
        .await?;

    Ok(())
}

async fn get_done_migrations(client: &PgClient) -> DbResult<Option<Vec<String>>> {
    match client
        .query_scalar("SELECT name FROM _migrations ORDER BY name ASC", &[])
        .await
    {
        Ok(list) => Ok(Some(list)),
        Err(err) if err.is_table_undefined_err() => Ok(None),
        Err(err) => Err(err),
    }
}

async fn apply_migration_in_transaction<M: Migration>(client: &mut PgClient) -> DbResult<()> {
    let trans = client.transaction().await?;
    println!("[db] Applying {}", M::name());

    match M::apply(&trans).await {
        Ok(()) => {
            println!("[db] Applied {} successfully.", M::name());
            Ok(())
        }
        Err(err) => {
            eprintln!("[db] Failed to apply {}; attempting rollback..", M::name());
            let _ = M::undo(&trans).await;
            Err(err)
        }
    }
}

define_migrations![];
