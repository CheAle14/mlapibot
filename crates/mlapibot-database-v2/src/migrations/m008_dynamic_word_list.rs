pub struct DynamicWordList;

impl super::Migration for DynamicWordList {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            CREATE TABLE subreddit_vague_words (
                subreddit_id    TEXT    NOT NULL REFERENCES subreddits(id),
                word            TEXT    NOT NULL,

                PRIMARY KEY (subreddit_id, word)
            );
        ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute("DROP TABLE subreddit_vague_words")
            .await?;

        Ok(())
    }
}
