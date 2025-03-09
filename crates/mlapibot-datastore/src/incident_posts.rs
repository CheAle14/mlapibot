use chrono::{DateTime, NaiveDateTime, Utc};

#[derive(Debug)]
pub struct IncidentPostLite {
    pub post_fullname: String,
    pub body_hash: String,
    pub updated_at: DateTime<Utc>,
}

impl IncidentPostLite {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let post_fullname = row.get(0)?;
        let body_hash = row.get(1)?;
        let updated_at: NaiveDateTime = row.get(2)?;

        Ok(Self {
            post_fullname,
            body_hash,
            updated_at: updated_at.and_utc(),
        })
    }
}
