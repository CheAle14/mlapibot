use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{ToSql, types::FromSql};

#[derive(Debug)]
pub struct IncidentPostLite {
    pub post_fullname: String,
    pub body_hash: String,
    pub updated_at: DateTime<Utc>,
    pub sticky_state: StickyState,
}

impl IncidentPostLite {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let post_fullname = row.get(0)?;
        let body_hash = row.get(1)?;
        let updated_at: NaiveDateTime = row.get(2)?;
        let sticky_state: StickyState = row.get(3)?;

        Ok(Self {
            post_fullname,
            body_hash,
            updated_at: updated_at.and_utc(),
            sticky_state,
        })
    }

    pub fn resolve(self, resolved_at: DateTime<Utc>) -> ResolvedIncidentPost {
        let Self {
            post_fullname,
            body_hash,
            updated_at,
            sticky_state,
        } = self;

        ResolvedIncidentPost {
            post_fullname,
            body_hash,
            updated_at,
            sticky_state,
            resolved_at,
        }
    }
}

#[derive(Debug)]
pub struct ResolvedIncidentPost {
    pub post_fullname: String,
    pub body_hash: String,
    pub updated_at: DateTime<Utc>,
    pub sticky_state: StickyState,
    pub resolved_at: DateTime<Utc>,
}

impl ResolvedIncidentPost {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let post_fullname = row.get(0)?;
        let body_hash = row.get(1)?;
        let updated_at: NaiveDateTime = row.get(2)?;
        let sticky_state: StickyState = row.get(3)?;
        let resolved_at: NaiveDateTime = row.get(4)?;

        Ok(Self {
            post_fullname,
            body_hash,
            updated_at: updated_at.and_utc(),
            sticky_state,
            resolved_at: resolved_at.and_utc(),
        })
    }
}

const NEVER_STICKIED: i64 = 0;
const STICKIED: i64 = 1;
const UNSTICKIED: i64 = 2;

#[derive(Debug, PartialEq)]
pub enum StickyState {
    /// Incident post was not stickied, per config.
    NeverStickied,
    /// Incident post is believed to still be sticked
    Stickied,
    /// Incident post has been unsticked, after delay from resolution.
    Unstickied,
}

impl FromSql for StickyState {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_i64()? {
            NEVER_STICKIED => Ok(Self::NeverStickied),
            STICKIED => Ok(Self::Stickied),
            UNSTICKIED => Ok(Self::Unstickied),
            v => Err(rusqlite::types::FromSqlError::OutOfRange(v)),
        }
    }
}

impl ToSql for StickyState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let v: i64 = match self {
            StickyState::NeverStickied => NEVER_STICKIED,
            StickyState::Stickied => STICKIED,
            StickyState::Unstickied => UNSTICKIED,
        };

        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Integer(v),
        ))
    }
}
