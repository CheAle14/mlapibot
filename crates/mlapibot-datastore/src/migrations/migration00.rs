use crate::MlapiDb;

pub struct Initial;

impl Initial {
    pub fn up(db: &mut MlapiDb) -> rusqlite::Result<()> {
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

            COMMIT;
        ",
        )?;

        Ok(())
    }
}
