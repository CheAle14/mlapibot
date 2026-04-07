/*
CREATE TABLE LiveIncidentPosts (
    incident_id         TEXT NOT NULL,
    fullname            TEXT NOT NULL,
    updated_at          TIMESTAMP NULL,

    PRIMARY KEY (incident_id)
);*/

use std::collections::HashSet;

use rusqlite::OptionalExtension;

use crate::{DateTimeUtc, MlapiDb};

pub struct LiveIncidentPost {
    pub incident_id: String,
    pub fullname: String,
    pub updated_at: Option<DateTimeUtc>,
    pub resolved_at: Option<DateTimeUtc>,
}

impl LiveIncidentPost {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let incident_id = row.get(0)?;
        let fullname = row.get(1)?;
        let updated_at = row.get(2)?;
        let resolved_at = row.get(3)?;

        Ok(Self {
            incident_id,
            fullname,
            updated_at,
            resolved_at,
        })
    }
}

impl MlapiDb {
    pub fn get_unresolved_live_incidents(&self) -> rusqlite::Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT IncidentId FROM LiveIncidentPosts WHERE ResolvedAt IS NULL")?;

        let mut ids = HashSet::new();
        for id in stmt.query_map((), |r| r.get(0))? {
            ids.insert(id?);
        }

        Ok(ids)
    }

    pub fn get_live_incident(
        &self,
        incident_id: &str,
    ) -> rusqlite::Result<Option<LiveIncidentPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT IncidentId, Fullname, UpdatedAt, ResolvedAt FROM LiveIncidentPosts WHERE IncidentId=?1",
        )?;

        stmt.query_row((incident_id,), LiveIncidentPost::from_row)
            .optional()
    }

    pub fn create_live_incident(
        &self,
        incident_id: &str,
        live_fullname: &str,
        updated_at: Option<DateTimeUtc>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO LiveIncidentPosts (IncidentId, Fullname, UpdatedAt) VALUES (?1,?2,?3)",
            (incident_id, live_fullname, updated_at),
        )?;

        Ok(())
    }

    pub fn update_live_incident(
        &self,
        incident_id: &str,
        updated_at: DateTimeUtc,
        resolved_at: Option<DateTimeUtc>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE LiveIncidentPosts SET UpdatedAt=?2, ResolvedAt=?3 WHERE IncidentId=?1",
            (incident_id, updated_at, resolved_at),
        )?;

        Ok(())
    }
}
