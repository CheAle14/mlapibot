pub struct AddFrontend;

impl super::Migration for AddFrontend {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            CREATE TABLE subreddits (
                id              TEXT        PRIMARY KEY NOT NULL,
                name            TEXT        NOT NULL,
                last_sync       TIMESTAMPTZ NOT NULL,
                options         JSONB       NOT NULL
            );

            CREATE TABLE subreddit_mods (
                subreddit_id    TEXT        REFERENCES subreddits(id) NOT NULL ,
                user_id         TEXT        NOT NULL,

                PRIMARY KEY (subreddit_id, user_id)
            );


            CREATE TABLE users (
                id              TEXT        PRIMARY KEY NOT NULL,
                name            TEXT        NOT NULL,

                admin           BOOL        NOT NULL DEFAULT false,
                cookie          TEXT        NULL,
                last_sync       TIMESTAMPTZ NULL DEFAULT CURRENT_TIMESTAMP
            );
        ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute("DROP TABLE frontend_users").await?;

        Ok(())
    }
}
