pub struct StaffReplyPrefix;

impl super::Migration for StaffReplyPrefix {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            r#"
        ALTER TABLE StaffReplyThreads
        ADD COLUMN Suffix TEXT NULL DEFAULT NULL;
        "#,
        )
    }
}
