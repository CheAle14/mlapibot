pub struct StaffReplyHash;

impl super::Migration for StaffReplyHash {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            r#"
        ALTER TABLE StaffReplyThreads
        ADD COLUMN Hash TEXT NOT NULL DEFAULT "";
        "#,
        )
    }
}
