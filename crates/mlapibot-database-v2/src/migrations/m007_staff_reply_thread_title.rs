pub struct StaffReplyThreadTitle;

impl super::Migration for StaffReplyThreadTitle {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            ALTER TABLE staff_reply_threads 
            ADD COLUMN title TEXT NULL
            ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            ALTER TABLE staff_reply_threads 
            DROP COLUMN title
            ",
        )
        .await?;

        Ok(())
    }
}
