pub struct StickyStoreId;

impl super::Migration for StickyStoreId {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            "
            ALTER TABLE IncidentPosts ADD COLUMN PriorStickyFullname TEXT DEFAULT NULL;
        ",
        )?;

        Ok(())
    }
}
