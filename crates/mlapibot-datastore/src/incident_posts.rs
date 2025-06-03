use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::ToSql;

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
        let sticky_state = StickyState::from_row(row, 3, 4)?;

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

        let sticky_state = StickyState::from_row(row, 3, 5)?;

        // since we set it as a DateTime<Utc>, sqlite seems to give it us back
        // in this same format.
        let resolved_at: DateTime<Utc> = row.get(4)?;

        Ok(Self {
            post_fullname,
            body_hash,
            updated_at: updated_at.and_utc(),
            sticky_state,
            resolved_at: resolved_at,
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
    Stickied {
        // The fullname of the post which was un-stickied to make way for this one.
        removed: Option<String>,
    },
    /// Incident post has been unsticked, after delay from resolution.
    Unstickied,
}

impl StickyState {
    fn from_row(
        row: &rusqlite::Row,
        self_idx: usize,
        removed_idx: usize,
    ) -> rusqlite::Result<Self> {
        match row.get::<_, i64>(self_idx)? {
            NEVER_STICKIED => Ok(StickyState::NeverStickied),
            STICKIED => {
                let removed = row.get(removed_idx)?;

                Ok(StickyState::Stickied { removed: removed })
            }
            UNSTICKIED => Ok(StickyState::Unstickied),
            v => Err(rusqlite::Error::IntegralValueOutOfRange(self_idx, v)),
        }
    }
}

impl ToSql for StickyState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let v: i64 = match self {
            StickyState::NeverStickied => NEVER_STICKIED,
            StickyState::Stickied { .. } => STICKIED,
            StickyState::Unstickied => UNSTICKIED,
        };

        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Integer(v),
        ))
    }
}
