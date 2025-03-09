use rusqlite::Row;

#[derive(Debug)]
pub struct Monitored {
    pub subreddit: String,
    pub post_fullname: String,
    pub state: MonitoredState,
}

pub(crate) const SEEN: i64 = 0;
pub(crate) const IGNORED: i64 = 1;
pub(crate) const ACTED: i64 = 2;

impl Monitored {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let subreddit = row.get("Subreddit")?;
        let post_fullname = row.get("PostFullname")?;

        let state = match row.get("State")? {
            SEEN => MonitoredState::Seen,
            IGNORED => MonitoredState::Ignored,
            ACTED => MonitoredState::Acted(MonitoredAction::from_row(row)?),
            v => return Err(rusqlite::Error::IntegralValueOutOfRange(2, v)),
        };

        Ok(Self {
            subreddit,
            post_fullname,
            state,
        })
    }
}

#[derive(Debug)]
pub enum MonitoredState {
    Seen,
    Ignored,
    Acted(MonitoredAction),
}

#[derive(Debug)]
pub struct MonitoredAction {
    /// Which analyzer triggered the response
    pub analyzer: String,
    /// If we replied, the fullname of our comment
    pub reply_fullname: Option<String>,
    /// Whether we reported the post
    pub reported: bool,
    /// Whether we removed the post
    pub removed: bool,
}

impl MonitoredAction {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let analyzer = row.get("Analyzer")?;
        let reply_fullname = row.get("ReplyFullname")?;
        let reported = row.get("Reported")?;
        let removed = row.get("Removed")?;

        Ok(Self {
            analyzer,
            reply_fullname,
            reported,
            removed,
        })
    }
}
