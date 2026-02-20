use crate::errors::{DbError, DbResult};
use postgres_types::{FromSql, ToSql};

#[derive(Debug, FromSql, ToSql)]
#[postgres(name = "monitorstate")]
#[postgres(rename_all = "snake_case")]
enum DbMonitorState {
    Seen,
    Ignored,
    Acted,
}

#[derive(Debug, PartialEq)]
pub enum MonitorState {
    Seen,
    Ignored,
    Acted {
        /// Which analyzer triggered the response
        analyzer: String,
        /// If we replied, the fullname of our comment
        reply_fullname: Option<String>,
        /// Whether we reported the post
        reported: bool,
        /// Whether we removed the post
        removed: bool,
        /// Whether our actions were incorrect
        mistaken: bool,
    },
}

pub trait MonitorRepo {
    type Error;

    async fn has_seen_item(&self, id: &str) -> Result<bool, Self::Error>;
    async fn delete_monitored(&self, id: &str) -> Result<bool, Self::Error>;
    async fn mark_item_seen(&self, id: &str, subreddit: &str) -> Result<(), Self::Error>;

    async fn update_item_monitor_state(
        &self,
        id: &str,
        state: MonitorState,
    ) -> Result<(), Self::Error>;

    async fn set_item_mistaken(&self, id: &str, mistaken: bool) -> Result<(), Self::Error>;
    async fn get_item_monitor_state(&self, id: &str) -> Result<MonitorState, Self::Error>;
}

impl MonitorRepo for crate::client::PgClient {
    type Error = DbError;

    async fn has_seen_item(&self, id: &str) -> DbResult<bool> {
        self.query_one_scalar(
            "SELECT EXISTS(SELECT 1 FROM monitored WHERE fullname=$1)",
            &[&id],
        )
        .await
    }

    async fn delete_monitored(&self, id: &str) -> DbResult<bool> {
        let n = self
            .execute("DELETE FROM monitored WHERE fullname=$1", &[&id])
            .await?;

        Ok(n == 1)
    }

    async fn mark_item_seen(&self, id: &str, subreddit: &str) -> DbResult<()> {
        self.execute(
            "INSERT INTO monitored (fullname, subreddit) VALUES ($1, $2)",
            &[&id, &subreddit],
        )
        .await?;

        Ok(())
    }

    async fn set_item_mistaken(&self, id: &str, mistaken: bool) -> DbResult<()> {
        self.execute(
            "UPDATE monitored SET mistaken=$2 WHERE fullname=$1",
            &[&id, &mistaken],
        )
        .await?;

        Ok(())
    }

    async fn update_item_monitor_state(&self, id: &str, state: MonitorState) -> DbResult<()> {
        match state {
            MonitorState::Seen => {
                self.execute(
                    "UPDATE monitored SET state='seen' WHERE fullname=$1",
                    &[&id],
                )
                .await?;
            }
            MonitorState::Ignored => {
                self.execute(
                    "UPDATE monitored SET state='ignored' WHERE fullname=$1",
                    &[&id],
                )
                .await?;
            }
            MonitorState::Acted {
                analyzer,
                reply_fullname,
                reported,
                removed,
                mistaken,
            } => {
                self.execute(
                    "UPDATE monitored SET
                        state='acted',
                        analyzer=$2,
                        reply=$3,
                        reported=$4,
                        removed=$5,
                        mistaken=$6
                    WHERE fullname=$1",
                    &[
                        &id,
                        &analyzer,
                        &reply_fullname,
                        &reported,
                        &removed,
                        &mistaken,
                    ],
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn get_item_monitor_state(&self, id: &str) -> DbResult<MonitorState> {
        self.query_one_map(
            "SELECT state, analyzer, reply, reported, removed, mistaken FROM monitored WHERE fullname=$1 LIMIT 1",
            &[&id],
            |row| {
                let state: DbMonitorState = row.get(0);

                match state {
                    DbMonitorState::Seen => Ok(MonitorState::Seen),
                    DbMonitorState::Ignored => Ok(MonitorState::Ignored),
                    DbMonitorState::Acted => Ok(MonitorState::Acted {
                        analyzer: row.get(1),
                        reply_fullname: row.get(2),
                        reported: row.get(3),
                        removed: row.get(4),
                        mistaken: row.get(5),
                    }),
                }
            },
        )
        .await
    }
}
