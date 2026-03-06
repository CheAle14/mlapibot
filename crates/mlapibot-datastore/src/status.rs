use std::collections::HashSet;

use rusqlite::OptionalExtension;

use crate::{
    MlapiDb,
    incident_posts::{IncidentPostLite, ResolvedIncidentPost},
};

impl MlapiDb {
    pub fn add_incident(
        &self,
        subreddit: &str,
        incident_id: &str,
        post_fullname: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO IncidentPosts (Subreddit, IncidentId, PostFullname) VALUES (?1,?2,?3)",
            (subreddit, incident_id, post_fullname),
        )?;

        Ok(())
    }

    pub fn get_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
    ) -> rusqlite::Result<Option<IncidentPostLite>> {
        let mut stmt = self.conn.prepare(
            "SELECT PostFullname, StickyState, PriorStickyFullname FROM IncidentPosts WHERE Subreddit=?1 AND IncidentId=?2",
        )?;

        stmt.query_row((subreddit, incident_id), IncidentPostLite::from_row)
            .optional()
    }

    pub fn get_incident_from_post(
        &self,
        fullname: &str,
    ) -> rusqlite::Result<Option<IncidentPostLite>> {
        let mut stmt = self.conn.prepare(
            "SELECT PostFullname, StickyState, PriorStickyFullname FROM IncidentPosts WHERE PostFullname=?1",
        )?;

        stmt.query_row((fullname,), IncidentPostLite::from_row)
            .optional()
    }

    pub fn get_unresolved_incident_posts(
        &self,
        subreddit: &str,
    ) -> rusqlite::Result<HashSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT IncidentId FROM IncidentPosts WHERE Subreddit=?1 AND ResolvedAt IS NULL",
        )?;

        let mut ids = HashSet::new();
        for id in stmt.query_map((subreddit,), |r| r.get(0))? {
            ids.insert(id?);
        }

        Ok(ids)
    }

    pub fn sticky_incident_post(
        &self,
        post_fullname: &str,
        prior_sticky: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET StickyState=1, PriorStickyFullname=?2 WHERE PostFullname=?1",
            (post_fullname, prior_sticky),
        )?;

        Ok(())
    }

    pub fn get_incident_posts_waiting_unsticky(
        &self,
    ) -> rusqlite::Result<Vec<ResolvedIncidentPost>> {
        // Since only the live incident post really tracks the resolved at,
        // each subreddit's resolved_at is no longer set.
        // Rather than also set it at the same time, it is easier to just
        // defer to the live incident table instead.
        // This means `IncidentPosts.ResolvedAt` effectively refers to
        // when the post was unstickied (or null, if it never was).
        let mut stmt = self.conn.prepare(
            "SELECT
                ic.PostFullname,
                ic.StickyState,
                ic.PriorStickyFullname,
                live.ResolvedAt
            FROM IncidentPosts ic
            INNER JOIN
                LiveIncidentPosts live ON live.IncidentId = ic.IncidentId
            WHERE
                live.ResolvedAt IS NOT NULL AND ic.StickyState == 1;",
        )?;

        let mut v = Vec::new();

        for value in stmt.query_map((), ResolvedIncidentPost::from_row)? {
            v.push(value?)
        }

        Ok(v)
    }

    pub fn set_incident_post_unstickied(&self, post_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET StickyState=2, PriorStickyFullname=NULL WHERE PostFullname=?1",
            (post_fullname,),
        )?;

        Ok(())
    }
}
