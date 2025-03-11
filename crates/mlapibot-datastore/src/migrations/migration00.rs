use crate::MlapiDb;

use super::Migration;

pub struct Initial;

impl Migration for Initial {
    fn apply(&self, db: &mut MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            "
            BEGIN;
            
            CREATE TABLE Monitored (
                Subreddit       TEXT    NOT NULL,
                PostFullname    TEXT    NOT NULL,
                CreatedAt       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
                State           INTEGER NOT NULL,

                Analzyer        TEXT        NULL,
                ReplyFullname   TEXT        NULL,
                Reported        BOOLEAN     NULL,
                Removed         BOOLEAN     NULL,

                Mistaken        BOOLEAN     DEFAULT FALSE,
                
                PRIMARY KEY (Subreddit, PostFullname)
            );

            CREATE TABLE IncidentPosts (
                IncidentId      TEXT    NOT NULL,
                Subreddit       TEXT    NOT NULL,
                PostFullname    TEXT    NOT NULL,
                UpdatedAt       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
                BodyHash        TEXT    NOT NULL,
                IsResolved      BOOLEAN DEFAULT FALSE,

                PRIMARY KEY (IncidentId, Subreddit)
            );

            COMMIT;
        ",
        )?;

        Ok(())
    }
}
