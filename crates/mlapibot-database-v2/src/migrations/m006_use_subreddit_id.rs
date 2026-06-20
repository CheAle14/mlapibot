pub struct UseSubredditId;

impl super::Migration for UseSubredditId {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            r#"
        ALTER TABLE staff_reply_threads
        ADD COLUMN subreddit_id     TEXT    NOT NULL DEFAULT '';

        UPDATE staff_reply_threads srt
        SET    subreddit_id = s.id
        FROM   subreddits s
        WHERE  srt.subreddit = s.name
        AND    srt.subreddit_id IS DISTINCT FROM s.id;

        ALTER TABLE staff_reply_threads
        DROP COLUMN subreddit;

        CREATE INDEX staff_reply_threads_subreddit_id_IDX ON staff_reply_threads USING HASH (subreddit_id);
        "#,
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
        DROP INDEX staff_reply_threads_subreddit_id_IDX;

        ALTER TABLE staff_reply_threads
        ADD COLUMN subreddit     TEXT    NOT NULL DEFAULT '';

        UPDATE staff_reply_threads srt
        SET    subreddit = s.name
        FROM   subreddits s
        WHERE  srt.subreddit_id = s.id
        AND    srt.subreddit IS DISTINCT FROM s.name;

        ALTER TABLE staff_reply_threads
        DROP COLUMN subreddit_id;
        ",
        )
        .await?;

        Ok(())
    }
}
