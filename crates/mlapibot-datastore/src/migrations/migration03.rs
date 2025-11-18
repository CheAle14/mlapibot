pub struct StatusLiveThread;

impl super::Migration for StatusLiveThread {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            "
            ALTER TABLE IncidentPosts DROP COLUMN BodyHash;
            ALTER TABLE IncidentPosts DROP COLUMN UpdatedAt;

            CREATE TABLE LiveIncidentPosts (
                IncidentId         TEXT NOT NULL,
                Fullname            TEXT NOT NULL,
                UpdatedAt          TIMESTAMP NULL,

                PRIMARY KEY (IncidentId)
            );
        ",
        )?;

        Ok(())
    }
}
