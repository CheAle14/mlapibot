/*
CREATE TABLE LiveIncidentPosts (
    incident_id         TEXT NOT NULL,
    fullname            TEXT NOT NULL,
    updated_at          TIMESTAMP NULL,

    PRIMARY KEY (incident_id)
);*/

use chrono::NaiveDateTime;
use rusqlite::OptionalExtension;

use crate::{DateTimeUtc, MlapiDb};

pub struct LiveIncidentPost {
    pub incident_id: String,
    pub fullname: String,
    pub updated_at: Option<DateTimeUtc>,
}

impl LiveIncidentPost {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let incident_id = row.get(0)?;
        let fullname = row.get(1)?;
        let updated_at = row.get(2)?;

        Ok(Self {
            incident_id,
            fullname,
            updated_at,
        })
    }
}

impl MlapiDb {
    pub fn get_live_incident(
        &self,
        incident_id: &str,
    ) -> rusqlite::Result<Option<LiveIncidentPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT IncidentId, Fullname, UpdatedAt FROM LiveIncidentPosts WHERE IncidentId=?1",
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
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE LiveIncidentPosts SET UpdatedAt=?2 WHERE IncidentId=?1",
            (incident_id, updated_at),
        )?;

        Ok(())
    }
}
