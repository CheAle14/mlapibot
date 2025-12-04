pub struct StaffReplyStoreSubreddit;

impl super::Migration for StaffReplyStoreSubreddit {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            r#"
            ALTER TABLE StaffReplyThreads
            ADD COLUMN Subreddit TEXT NOT NULL DEFAULT "<fixme>" COLLATE NOCASE;

            ALTER TABLE StaffReplyThreads
            ADD COLUMN CreatedAt TIMESTAMP NOT NULL DEFAULT "2025-12-04T00:00:00Z";
        "#,
        )
    }
}
