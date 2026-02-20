use std::collections::HashSet;

use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;

use crate::{
    DateTimeUtc,
    client::PgClient,
    errors::{DbError, DbResult},
};

/// Partial information about a subreddit's incident post.
#[derive(Debug, PartialEq)]
pub struct IncidentPostLite {
    pub post_fullname: String,
    pub sticky_state: StickyState,
}

impl IncidentPostLite {
    fn from_row(row: Row) -> DbResult<Self> {
        let post_fullname: String = row.get("fullname");
        let sticky_state = StickyState::from_row(&row)?;

        Ok(Self {
            post_fullname,
            sticky_state,
        })
    }
}

/// A subreddit's incident post referring to a resolved incident.
#[derive(Debug, PartialEq)]
pub struct ResolvedIncidentPost {
    pub post_fullname: String,
    pub sticky_state: StickyState,
    pub resolved_at: DateTimeUtc,
}

impl ResolvedIncidentPost {
    fn from_row(row: Row) -> DbResult<Self> {
        let post_fullname: String = row.get("fullname");
        let resolved_at = row.get("resolved_at");
        let sticky_state = StickyState::from_row(&row)?;

        Ok(Self {
            post_fullname,
            sticky_state,
            resolved_at,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum StickyState {
    /// Incident post was not stickied, per config.
    NeverStickied,
    /// Incident post is believed to still be sticked
    Stickied {
        /// The fullname of the post which was un-stickied to make way for this one.
        removed: Option<String>,
    },
    /// Incident post has been unsticked, after delay from resolution.
    Unstickied { unstickied_at: DateTimeUtc },
}

impl StickyState {
    fn from_row(row: &Row) -> DbResult<Self> {
        println!("has columns:");
        for col in row.columns() {
            println!("- {} {:?}", col.name(), col.type_());
        }

        let sticky_state: DbStickyState = row.get("sticky_state");

        match sticky_state {
            DbStickyState::Never => Ok(StickyState::NeverStickied),
            DbStickyState::Currently => Ok(StickyState::Stickied {
                removed: row.get("prior_sticky"),
            }),
            DbStickyState::Undone => Ok(StickyState::Unstickied {
                unstickied_at: row.get("unstickied_at"),
            }),
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "incidentstickystate", rename_all = "lowercase")]
enum DbStickyState {
    Never,
    Currently,
    Undone,
}

/// A status incident which an associated live-updating thread.
#[derive(Debug, PartialEq)]
pub struct StatusIncident {
    pub incident_id: String,
    pub updated_at: Option<DateTimeUtc>,
    pub resolved_at: Option<DateTimeUtc>,

    pub live_fullname: String,
}

impl StatusIncident {
    fn from_row(row: Row) -> DbResult<Self> {
        Ok(Self {
            incident_id: row.get("incident_id"),
            updated_at: row.get("updated_at"),
            resolved_at: row.get("resolved_at"),
            live_fullname: row.get("live_fullname"),
        })
    }
}

pub trait IncidentRepo {
    type Error;

    async fn create_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
        post_fullname: &str,
    ) -> Result<(), Self::Error>;

    async fn get_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
    ) -> Result<Option<IncidentPostLite>, Self::Error>;

    async fn get_incident_post_by_id(
        &self,
        fullname: &str,
    ) -> Result<Option<IncidentPostLite>, Self::Error>;

    async fn get_stickied_incident_posts(
        &self,
        subreddit: &str,
    ) -> Result<HashSet<String>, Self::Error>;

    async fn sticky_incident_post(
        &self,
        post_fullname: &str,
        prior_sticky: Option<&str>,
    ) -> Result<(), Self::Error>;

    async fn unsticky_incident_post(
        &self,
        post_fullname: &str,
        unstickied_at: DateTimeUtc,
    ) -> Result<(), Self::Error>;

    async fn get_resolved_incidents_still_stickied(
        &self,
    ) -> Result<Vec<ResolvedIncidentPost>, Self::Error>;

    async fn get_unresolved_status_incidents(&self) -> Result<HashSet<String>, Self::Error>;

    async fn get_status_incident_by_id(
        &self,
        incident_id: &str,
    ) -> Result<Option<StatusIncident>, Self::Error>;

    async fn create_status_incident(
        &self,
        incident_id: &str,
        live_fullname: &str,
        updated_at: Option<DateTimeUtc>,
    ) -> Result<(), Self::Error>;

    async fn update_status_incident(
        &self,
        incident_id: &str,
        updated_at: DateTimeUtc,
        resolved_at: Option<DateTimeUtc>,
    ) -> Result<(), Self::Error>;
}

impl IncidentRepo for PgClient {
    type Error = DbError;

    async fn create_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
        post_fullname: &str,
    ) -> Result<(), Self::Error> {
        self.execute(
            "INSERT INTO incident_sub_posts (subreddit, incident_id, fullname) VALUES ($1, $2, $3)",
            &[&subreddit, &incident_id, &post_fullname],
        )
        .await?;

        Ok(())
    }

    async fn get_incident_post(
        &self,
        subreddit: &str,
        incident_id: &str,
    ) -> Result<Option<IncidentPostLite>, Self::Error> {
        self.query_opt_map(
            "
            SELECT fullname, sticky_state, prior_sticky, unstickied_at
            FROM incident_sub_posts
            WHERE subreddit=$1 AND incident_id=$2",
            &[&subreddit, &incident_id],
            IncidentPostLite::from_row,
        )
        .await
    }

    async fn get_incident_post_by_id(
        &self,
        fullname: &str,
    ) -> Result<Option<IncidentPostLite>, Self::Error> {
        self.query_opt_map(
            "
            SELECT fullname, sticky_state, prior_sticky, unstickied_at
            FROM incident_sub_posts
            WHERE fullname=$1",
            &[&fullname],
            IncidentPostLite::from_row,
        )
        .await
    }

    async fn get_stickied_incident_posts(
        &self,
        subreddit: &str,
    ) -> Result<HashSet<String>, Self::Error> {
        let items: Vec<String> = self
            .query_scalar(
                "
            SELECT incident_id
            FROM incident_sub_posts
            WHERE subreddit=$1 AND unstickied_at IS NULL",
                &[&subreddit],
            )
            .await?;

        Ok(items.into_iter().collect())
    }

    async fn sticky_incident_post(
        &self,
        post_fullname: &str,
        prior_sticky: Option<&str>,
    ) -> Result<(), Self::Error> {
        self.execute(
            "
            UPDATE incident_sub_posts
            SET
                sticky_state='currently',
                unstickied_at=NULL,
                prior_sticky=$2
            WHERE
                fullname=$1
            ",
            &[&post_fullname, &prior_sticky],
        )
        .await?;

        Ok(())
    }

    async fn unsticky_incident_post(
        &self,
        post_fullname: &str,
        unstickied_at: DateTimeUtc,
    ) -> Result<(), Self::Error> {
        self.execute(
            "
            UPDATE incident_sub_posts
            SET
                sticky_state='undone',
                unstickied_at=$2
            WHERE
                fullname=$1
            ",
            &[&post_fullname, &unstickied_at],
        )
        .await?;

        Ok(())
    }

    async fn get_resolved_incidents_still_stickied(
        &self,
    ) -> Result<Vec<ResolvedIncidentPost>, Self::Error> {
        // `status_incidents` tracks whether the incident itself is resolved,
        // whilst each `incident_sub_posts` tracks whether that post has been
        // unstickied yet.
        // We want to pull resolved incidents that are still stickied to see
        // whether we should be unstickying them, as there's a variable delay
        // after they're marked as resolved before we actually unsticky.
        self.query_map(
            "SELECT
                ic.fullname,
                ic.sticky_state,
                ic.prior_sticky,
                ic.unstickied_at,
                live.resolved_at
            FROM incident_sub_posts ic
            INNER JOIN
                status_incidents live ON live.incident_id = ic.incident_id
            WHERE
                live.resolved_at IS NOT NULL AND ic.sticky_state = 'currently';",
            &[],
            ResolvedIncidentPost::from_row,
        )
        .await
    }

    async fn get_unresolved_status_incidents(&self) -> Result<HashSet<String>, Self::Error> {
        let names: Vec<String> = self
            .query_scalar(
                "
            SELECT incident_id
            FROM status_incidents
            WHERE resolved_at IS NULL",
                &[],
            )
            .await?;

        Ok(names.into_iter().collect())
    }

    async fn get_status_incident_by_id(
        &self,
        incident_id: &str,
    ) -> Result<Option<StatusIncident>, Self::Error> {
        self.query_opt_map(
            "
            SELECT incident_id, updated_at, resolved_at, live_fullname
            FROM status_incidents
            WHERE incident_id=$1",
            &[&incident_id],
            StatusIncident::from_row,
        )
        .await
    }

    async fn create_status_incident(
        &self,
        incident_id: &str,
        live_fullname: &str,
        updated_at: Option<DateTimeUtc>,
    ) -> Result<(), Self::Error> {
        self.execute("INSERT INTO status_incidents (incident_id, live_fullname, updated_at) VALUES ($1, $2, $3)", &[&incident_id, &live_fullname, &updated_at]).await?;

        Ok(())
    }

    async fn update_status_incident(
        &self,
        incident_id: &str,
        updated_at: DateTimeUtc,
        resolved_at: Option<DateTimeUtc>,
    ) -> Result<(), Self::Error> {
        self.execute(
            "
            UPDATE status_incidents
            SET
                updated_at=$2,
                resolved_at=$3
            WHERE
                incident_id=$1",
            &[&incident_id, &updated_at, &resolved_at],
        )
        .await?;

        Ok(())
    }
}
