use crate::MlapiDb;

impl MlapiDb {
    pub fn has_seen(&self, post_fullname: &str) -> rusqlite::Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM Monitored WHERE PostFullname=?1")?;

        let r = stmt.query_row((post_fullname,), |r| r.get::<_, u32>(0))?;

        Ok(r > 0)
    }

    pub fn set_seen(&self, subreddit: &str, post_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO Monitored (Subreddit, PostFullname, State) VALUES (?1, ?2, ?3)",
            (subreddit, post_fullname, crate::monitored::SEEN),
        )?;

        Ok(())
    }

    pub fn set_ignored(&self, post_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE Monitored SET State = ?1 WHERE PostFullname = ?2",
            (crate::monitored::IGNORED, post_fullname),
        )?;

        Ok(())
    }

    pub fn set_analyzed(
        &self,
        post_fullname: &str,
        analyzer: &str,
        reply: Option<&str>,
        reported: bool,
        removed: bool,
    ) -> rusqlite::Result<()> {
        match reply {
            None => self.conn.execute(
                "
                UPDATE Monitored
                SET 
                    State = ?1,
                    Analzyer = ?2,
                    Reported = ?3,
                    Removed = ?4
                WHERE PostFullname=?5;
                ",
                (
                    crate::monitored::ACTED,
                    analyzer,
                    reported,
                    removed,
                    post_fullname,
                ),
            ),
            Some(reply) => self.conn.execute(
                "
                UPDATE Monitored
                SET 
                    State = ?1,
                    Analzyer = ?2,
                    ReplyFullname = ?3,
                    Reported = ?4,
                    Removed = ?5
                WHERE PostFullname = ?6;
                ",
                (
                    crate::monitored::ACTED,
                    analyzer,
                    reply,
                    reported,
                    removed,
                    post_fullname,
                ),
            ),
        }?;

        Ok(())
    }

    pub fn set_mistaken(&self, comment_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE Monitored SET Mistaken = TRUE WHERE ReplyFullname = ?1",
            (comment_fullname,),
        )?;

        Ok(())
    }
}
