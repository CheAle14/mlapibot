pub struct InitMonitored;

impl super::Migration for InitMonitored {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            CREATE TYPE MonitorState AS ENUM ('seen', 'ignored', 'acted');

            CREATE TABLE monitored (
            fullname    TEXT            PRIMARY KEY,

            subreddit   TEXT            NOT NULL,
            created_at  TIMESTAMPTZ     NOT NULL DEFAULT CURRENT_TIMESTAMP,
            state       MonitorState    NOT NULL DEFAULT 'seen',
            analyzer    TEXT            NULL,
            reply       TEXT            NULL,
            reported    BOOL            NULL,
            removed     BOOL            NULL,
            mistaken    BOOL            NULL
        );",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            DROP TABLE monitored;
            DROP TYPE MonitorState;
            ",
        )
        .await?;

        Ok(())
    }
}
