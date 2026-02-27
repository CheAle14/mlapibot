use postgres_types::Json;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

use crate::{DateTimeUtc, errors::DbResult};

pub trait SubredditsRepo {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn fetch_all_subreddits(&self) -> Result<Vec<Subreddit>, Self::Error>;
    async fn create_subreddit(&self, sub: &Subreddit) -> Result<(), Self::Error>;

    async fn get_subreddit_moderators(&self, id: &str) -> Result<Vec<String>, Self::Error>;
    async fn set_subreddit_moderators(
        &mut self,
        id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error>;
}

impl SubredditsRepo for crate::client::PgClient {
    type Error = crate::errors::DbError;

    async fn fetch_all_subreddits(&self) -> Result<Vec<Subreddit>, Self::Error> {
        self.query_map(
            "
            SELECT *
            FROM subreddits",
            &[],
            Subreddit::from_row,
        )
        .await
    }

    async fn create_subreddit(&self, sub: &Subreddit) -> Result<(), Self::Error> {
        self.execute(
            "
            INSERT INTO
            subreddits (id, name, last_sync, seq_num, mod_json_schema,
            mod_scams, mod_ai_slop, mod_staff_reply, mod_status, mod_related_title)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &sub.id,
                &sub.name,
                &sub.last_sync,
                &sub.seq_num,
                &sub.mod_json_schema,
                &Json(&sub.mod_scams),
                &Json(&sub.mod_ai_slop),
                &Json(&sub.mod_staff_reply),
                &Json(&sub.mod_status),
                &Json(&sub.mod_related_title),
            ],
        )
        .await?;

        Ok(())
    }

    async fn get_subreddit_moderators(&self, id: &str) -> Result<Vec<String>, Self::Error> {
        self.query_scalar(
            "
            SELECT user_id
            FROM subreddit_mods
            WHERE subreddit_id=$1",
            &[&id],
        )
        .await
    }

    async fn set_subreddit_moderators(
        &mut self,
        id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error> {
        use std::fmt::Write;

        let db = self.transaction().await?;

        db.execute("DELETE FROM subreddit_mods WHERE subreddit_id=$1", &[&id])
            .await?;

        let mut query = String::from("INSERT INTO subreddit_mods (subreddit_id, user_id) VALUES ");
        let mut params: Vec<&(dyn postgres_types::ToSql + Sync)> = vec![&id];

        for user_id in user_ids {
            params.push(&*user_id);
            let _ = writeln!(query, "($1, ${}),", params.len());
        }

        query.replace_last(',', ";");

        db.execute(&query, &params).await?;

        db.commit().await?;

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Subreddit {
    pub id: String,
    pub name: String,
    pub last_sync: DateTimeUtc,
    pub seq_num: i32,
    pub mod_json_schema: i32,
    pub mod_scams: ScamsModule,
    pub mod_ai_slop: AiSlopModule,
    pub mod_staff_reply: StaffReplyModule,
    pub mod_status: StatusModule,
    pub mod_related_title: RelatedTitleModule,
}

impl Subreddit {
    fn from_row(row: Row) -> DbResult<Self> {
        let id = row.get("id");
        let name = row.get("name");
        let last_sync = row.get("last_sync");
        let seq_num = row.get("seq_num");
        let mod_json_schema = row.get("mod_json_schema");

        let mod_scams: Json<ScamsModule> = row.get("mod_scams");
        let mod_ai_slop: Json<AiSlopModule> = row.get("mod_ai_slop");
        let mod_staff_reply: Json<StaffReplyModule> = row.get("mod_staff_reply");
        let mod_status: Json<StatusModule> = row.get("mod_status");
        let mod_related_title: Json<RelatedTitleModule> = row.get("mod_related_title");

        Ok(Self {
            id,
            name,
            last_sync,
            seq_num,
            mod_json_schema,
            mod_scams: mod_scams.0,
            mod_ai_slop: mod_ai_slop.0,
            mod_staff_reply: mod_staff_reply.0,
            mod_status: mod_status.0,
            mod_related_title: mod_related_title.0,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ScamsModule {
    pub enabled: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AiSlopModule {
    pub enabled: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StaffReplyModule {
    pub enabled: bool,
    pub flair_id: String,
    pub css_class: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusModule {
    pub enabled: bool,
    pub min_impact: statuspage::incident::IncidentImpact,
    pub sticky: Option<StatusStickyConfig>,
    pub distinguish: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusStickyConfig {
    pub replace_sticky: Option<String>,
    pub comment_threshold: i32,
    pub delay_minor_mins: i32,
    pub delay_major_mins: i32,
    pub min_impact: Option<statuspage::incident::IncidentImpact>,
    pub only_for: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RelatedTitleModule {
    pub enabled: bool,
}
