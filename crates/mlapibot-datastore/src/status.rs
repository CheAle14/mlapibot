use std::collections::HashSet;

use chrono::{DateTime, Utc};
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
        body_hash: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute("INSERT INTO IncidentPosts (Subreddit, IncidentId, PostFullname, BodyHash) VALUES (?1,?2,?3,?4)", (subreddit, incident_id, post_fullname, body_hash))?;

        Ok(())
    }

    pub fn get_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
    ) -> rusqlite::Result<Option<IncidentPostLite>> {
        let mut stmt = self.conn.prepare(
            "SELECT PostFullname, BodyHash, UpdatedAt, StickyState FROM IncidentPosts WHERE Subreddit=?1 AND IncidentId=?2",
        )?;

        stmt.query_row((subreddit, incident_id), IncidentPostLite::from_row)
            .optional()
    }

    pub fn update_incident_post(
        &self,
        post_fullname: &str,
        body_hash: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET BodyHash=?1, UpdatedAt=CURRENT_TIMESTAMP WHERE PostFullname=?2",
            (body_hash, post_fullname),
        )?;

        Ok(())
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

    pub fn sticky_incident_post(&self, post_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET StickyState=1 WHERE PostFullname=?1",
            (post_fullname,),
        )?;

        Ok(())
    }

    pub fn resolve_incident_post(
        &self,
        post_fullname: &str,
        resolved_at: DateTime<Utc>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET ResolvedAt=?1 WHERE PostFullname=?2",
            (resolved_at, post_fullname),
        )?;

        Ok(())
    }

    pub fn get_incident_posts_waiting_unsticky(
        &self,
    ) -> rusqlite::Result<Vec<ResolvedIncidentPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT PostFullname, BodyHash, UpdatedAt, StickyState, ResolvedAt FROM IncidentPosts WHERE ResolvedAt IS NOT NULL AND StickyState==1",
        )?;

        let mut v = Vec::new();

        for value in stmt.query_map((), ResolvedIncidentPost::from_row)? {
            v.push(value?)
        }

        Ok(v)
    }

    pub fn set_incident_post_unstickied(&self, post_fullname: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE IncidentPosts SET StickyState=2 WHERE PostFullname=?1",
            (post_fullname,),
        )?;

        Ok(())
    }
}
