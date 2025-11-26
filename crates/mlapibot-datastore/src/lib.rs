use std::path::Path;

pub mod incident_posts;
pub mod live_incident_posts;
pub mod monitored;
pub mod staff_replies;

mod seen;
mod status;

mod migrations;

pub(crate) type DateTimeUtc = chrono::DateTime<chrono::Utc>;

pub struct MlapiDb {
    pub(crate) conn: rusqlite::Connection,
}

impl MlapiDb {
    pub fn new(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;

        let mut this = Self { conn };

        migrations::ensure_updated(&mut this)?;

        Ok(this)
    }

    pub fn get_migration_version(&self) -> rusqlite::Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT user_version FROM pragma_user_version")?;
        stmt.query_row((), |r| r.get(0))
    }

    pub fn set_migration_version(&self, v: u32) -> rusqlite::Result<()> {
        self.conn.pragma_update(None, "user_version", v)
    }
}

pub(crate) trait QueryCollect {
    fn query_collect<T, P, F>(&mut self, params: P, f: F) -> Result<Vec<T>, rusqlite::Error>
    where
        P: rusqlite::Params,
        F: FnMut(&rusqlite::Row) -> Result<T, rusqlite::Error>;
}

impl<'conn> QueryCollect for rusqlite::Statement<'conn> {
    fn query_collect<T, P, F>(&mut self, params: P, f: F) -> Result<Vec<T>, rusqlite::Error>
    where
        P: rusqlite::Params,
        F: FnMut(&rusqlite::Row) -> Result<T, rusqlite::Error>,
    {
        let mut vec = Vec::new();
        for item in self.query_map(params, f)? {
            vec.push(item?);
        }
        Ok(vec)
    }
}
