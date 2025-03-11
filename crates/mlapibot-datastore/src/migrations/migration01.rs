use super::Migration;

pub struct ResolvedAt;

impl Migration for ResolvedAt {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            "
            BEGIN;

            ALTER TABLE IncidentPosts ADD COLUMN ResolvedAt TIMESTAMP NULL;
            ALTER TABLE IncidentPosts ADD COLUMN StickyState INTEGER NOT NULL DEFAULT 0;

            ALTER TABLE IncidentPosts DROP COLUMN IsResolved;
            
            COMMIT;
        ",
        )?;

        Ok(())
    }
}
