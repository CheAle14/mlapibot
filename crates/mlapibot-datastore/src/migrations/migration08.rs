pub struct LiveIncidentPostResolved;

impl super::Migration for LiveIncidentPostResolved {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            r#"
        ALTER TABLE LiveIncidentPosts
        ADD COLUMN ResolvedAt TIMESTAMP NULL DEFAULT NULL;
        "#,
        )
    }
}
