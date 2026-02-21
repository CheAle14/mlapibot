pub struct InitStaffReplies;

impl super::Migration for InitStaffReplies {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            r#"
        CREATE TABLE staff_reply_threads (
            post_id         TEXT        PRIMARY KEY,
            subreddit       TEXT        NOT NULL,
            our_comment_id  TEXT        NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            hash            TEXT        NOT NULL,
            suffix          TEXT        NULL
        );

        CREATE TABLE staff_replies (
            comment_id      TEXT        PRIMARY KEY,
            post_id         TEXT        NOT NULL,
            last_updated    TIMESTAMPTZ NOT NULL    DEFAULT CURRENT_TIMESTAMP,
            author_name     TEXT        NOT NULL,
            content         TEXT        NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL    DEFAULT CURRENT_TIMESTAMP
        );
        "#,
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            DROP TABLE staff_replies
            DROP TABLE staff_reply_threads;
        ",
        )
        .await?;

        Ok(())
    }
}
