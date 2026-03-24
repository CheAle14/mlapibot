pub struct ScheduledPosts;

impl super::Migration for ScheduledPosts {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            r"
            CREATE TABLE subreddit_posts (
                id          SERIAL          PRIMARY KEY,
                subreddit   TEXT            NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
                reddit_id   TEXT            NULL,

                updated_at  TIMESTAMPTZ     NOT NULL DEFAULT CURRENT_TIMESTAMP,
                synced_at   TIMESTAMPTZ     NULL,

                title       TEXT            NOT NULL,
                flair_id    TEXT            NULL,
                flair_text  TEXT            NULL,
                sticky      INTEGER         NOT NULL,
                distinguish BOOLEAN         NOT NULL,
                lock        BOOLEAN         NOT NULL,

                content     TEXT            NULL
            );
        ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute("DROP TABLE subreddit_posts").await?;

        Ok(())
    }
}
