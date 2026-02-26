use tokio_postgres::Transaction;

use crate::{client::PgClient, errors::DbResult};

pub trait Migration {
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
              let name = concat!(stringify!($module), "::", stringify!($struct));
              if !migrations.iter().any(|i| i == name) {
                  apply_migration_in_transaction::<$module::$struct>(name, client).await?;
              }
            )*

            Ok(())
        }

        pub async fn drop_migrations(client: &mut crate::client::PgClient, mut count: usize) -> DbResult<()> {
            let mut migrations = match get_done_migrations(&client).await? {
                Some(m) => m,
                None => {
                    println!("[db] No migrations table, nothing to undo.");
                    return Ok(());
                }
            };

            while count > 0 && let Some(last) = migrations.pop() {
                $(
                  let name = concat!(stringify!($module), "::", stringify!($struct));
                  if last == name {
                      drop_migration_in_transaction::<$module::$struct>(name, client).await?;
                      count -= 1;
                      continue;
                  }
                )*

                eprintln!("[db] Unrecognised migration, don't know how to drop: {last:?}");
            }


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

async fn apply_migration_in_transaction<M: Migration>(
    name: &str,
    client: &mut PgClient,
) -> DbResult<()> {
    let trans = client.transaction().await?;
    println!("[db] Applying {name}");

    match M::apply(&trans).await {
        Ok(()) => {
            trans
                .execute("INSERT INTO _migrations (name) VALUES ($1)", &[&name])
                .await?;

            trans.commit().await?;
            println!("[db] Applied {name} successfully.");
            Ok(())
        }
        Err(err) => {
            eprintln!("[db] Failed to apply {name}; attempting rollback..");
            if let Err(t_err) = trans.rollback().await {
                eprintln!("[db] Failed to rollback: {t_err}");
            }
            Err(err)
        }
    }
}

async fn drop_migration_in_transaction<M: Migration>(
    name: &str,
    client: &mut PgClient,
) -> DbResult<()> {
    let trans = client.transaction().await?;
    println!("[db] Dropping {name}");

    match M::undo(&trans).await {
        Ok(()) => {
            trans
                .execute("DELETE FROM _migrations WHERE name=$1", &[&name])
                .await?;

            trans.commit().await?;
            println!("[db] Dropped {name} successfully.");
            Ok(())
        }
        Err(err) => {
            eprintln!("[db] Failed to drop {name}; attempting rollback..");
            if let Err(t_err) = trans.rollback().await {
                eprintln!("[db] Failed to rollback: {t_err}");
            }
            Err(err)
        }
    }
}

define_migrations![
    m001_init_monitored::InitMonitored,
    m002_init_staff_replies::InitStaffReplies,
    m003_init_incidents::InitIncidents,
    m004_add_frontend::AddFrontend,
];
