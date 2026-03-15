use std::{borrow::Borrow, collections::HashMap};

use postgres_types::Json;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

use mlapibot_common::matchers::Matchers;

use crate::{DateTimeUtc, errors::DbResult};

pub trait SubredditsRepo {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn fetch_all_subreddits(&self) -> Result<Vec<Subreddit>, Self::Error>;
    async fn create_subreddit(&self, sub: &Subreddit) -> Result<(), Self::Error>;

    async fn set_subreddit_enabled(&self, id: &str, enabled: bool) -> Result<(), Self::Error>;

    async fn get_subreddit_moderators(&self, id: &str) -> Result<Vec<String>, Self::Error>;
    async fn set_subreddit_moderators(
        &mut self,
        id: &str,
        usernames: &[&str],
    ) -> Result<(), Self::Error>;

    async fn get_subreddit_templates(&self, id: &str) -> Result<Vec<ReplyTemplate>, Self::Error>;
    async fn get_subreddit_scams(&self, id: &str) -> Result<Vec<Scam>, Self::Error>;
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
        let Subreddit {
            id,
            name,
            enabled,
            last_sync,
            seq_num,
            mod_json_schema,
            removal_reasons,
            mod_scams,
            mod_ai_slop,
            mod_staff_reply,
            mod_status,
            mod_related_title,
            mod_complex_comments,
            mod_comments_code,
            mod_comments_cdn,
        } = sub;

        self.execute(
            "
            INSERT INTO
            subreddits (
                id, name, enabled, last_sync, seq_num, mod_json_schema, removal_reasons,
                mod_scams, mod_ai_slop, mod_staff_reply, mod_status, mod_related_title,
                mod_complex_comments,
                mod_comments_code,
                mod_comments_cdn
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            &[
                &id,
                &name,
                &enabled,
                &last_sync,
                &seq_num,
                &mod_json_schema,
                &Json(&removal_reasons),
                &Json(&mod_scams),
                &Json(&mod_ai_slop),
                &Json(&mod_staff_reply),
                &Json(&mod_status),
                &Json(&mod_related_title),
                &Json(&mod_complex_comments),
                &Json(&mod_comments_code),
                &Json(&mod_comments_cdn),
            ],
        )
        .await?;

        Ok(())
    }

    async fn get_subreddit_moderators(&self, id: &str) -> Result<Vec<String>, Self::Error> {
        self.query_scalar(
            "
            SELECT username
            FROM subreddit_mods
            WHERE subreddit_id=$1",
            &[&id],
        )
        .await
    }

    async fn set_subreddit_moderators(
        &mut self,
        id: &str,
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        use std::fmt::Write;

        let db = self.transaction().await?;

        db.execute("DELETE FROM subreddit_mods WHERE subreddit_id=$1", &[&id])
            .await?;

        let mut query = String::from("INSERT INTO subreddit_mods (subreddit_id, username) VALUES ");
        let mut params: Vec<&(dyn postgres_types::ToSql + Sync)> = vec![&id];

        for username in usernames {
            params.push(&*username);
            let _ = writeln!(query, "($1, ${}),", params.len());
        }

        query.replace_last(',', ";");

        db.execute(&query, &params).await?;

        db.execute(
            "UPDATE subreddits SET last_sync=CURRENT_TIMESTAMP WHERE id=$1",
            &[&id],
        )
        .await?;

        db.commit().await?;

        Ok(())
    }

    async fn set_subreddit_enabled(&self, id: &str, enabled: bool) -> Result<(), Self::Error> {
        self.execute(
            "UPDATE subreddits SET enabled=$2 WHERE id=$1",
            &[&id, &enabled],
        )
        .await?;

        Ok(())
    }

    async fn get_subreddit_templates(&self, id: &str) -> Result<Vec<ReplyTemplate>, Self::Error> {
        self.query_map(
            "
            SELECT * FROM subreddit_templates
            WHERE subreddit_id=$1",
            &[&id],
            ReplyTemplate::from_row,
        )
        .await
    }

    async fn get_subreddit_scams(&self, id: &str) -> Result<Vec<Scam>, Self::Error> {
        self.query_map(
            "
            SELECT * FROM subreddit_scam_rules
            WHERE subreddit_id=$1",
            &[&id],
            Scam::from_row,
        )
        .await
    }
}

#[derive(Debug, PartialEq)]
pub struct Subreddit {
    pub id: String,
    pub name: String,
    /// False if we are no longer a moderator of this subreddit.
    pub enabled: bool,
    pub last_sync: DateTimeUtc,
    pub seq_num: i32,
    pub mod_json_schema: i32,
    pub removal_reasons: RemovalReasonsMap,
    pub mod_scams: ScamsModule,
    pub mod_ai_slop: AiSlopModule,
    pub mod_staff_reply: StaffReplyModule,
    pub mod_status: StatusModule,
    pub mod_related_title: RelatedTitleModule,
    pub mod_complex_comments: ComplexCommentsModule,
    pub mod_comments_code: CommentsCodeModule,
    pub mod_comments_cdn: CommentsCdnModule,
}

impl Subreddit {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            enabled: true,
            last_sync: DateTimeUtc::UNIX_EPOCH,
            seq_num: 0,
            mod_json_schema: 0,
            removal_reasons: RemovalReasonsMap::default(),
            mod_scams: ScamsModule::default(),
            mod_ai_slop: AiSlopModule::default(),
            mod_staff_reply: StaffReplyModule::default(),
            mod_status: StatusModule::default(),
            mod_related_title: RelatedTitleModule::default(),
            mod_complex_comments: ComplexCommentsModule::default(),
            mod_comments_code: CommentsCodeModule::default(),
            mod_comments_cdn: CommentsCdnModule::default(),
        }
    }

    fn from_row(row: Row) -> DbResult<Self> {
        let id = row.get("id");
        let name = row.get("name");
        let enabled = row.get("enabled");
        let last_sync = row.get("last_sync");
        let seq_num = row.get("seq_num");

        let mod_json_schema = row.get("mod_json_schema");

        let removal_reasons: Json<RemovalReasonsMap> = row.get("removal_reasons");
        let mod_scams: Json<ScamsModule> = row.get("mod_scams");
        let mod_ai_slop: Json<AiSlopModule> = row.get("mod_ai_slop");
        let mod_staff_reply: Json<StaffReplyModule> = row.get("mod_staff_reply");
        let mod_status: Json<StatusModule> = row.get("mod_status");
        let mod_related_title: Json<RelatedTitleModule> = row.get("mod_related_title");
        let mod_complex_comments: Json<ComplexCommentsModule> = row.get("mod_complex_comments");
        let mod_comments_code: Json<CommentsCodeModule> = row.get("mod_comments_code");
        let mod_comments_cdn: Json<CommentsCdnModule> = row.get("mod_comments_cdn");

        Ok(Self {
            id,
            name,
            enabled,
            last_sync,
            seq_num,
            mod_json_schema,
            removal_reasons: removal_reasons.0,
            mod_scams: mod_scams.0,
            mod_ai_slop: mod_ai_slop.0,
            mod_staff_reply: mod_staff_reply.0,
            mod_status: mod_status.0,
            mod_related_title: mod_related_title.0,
            mod_complex_comments: mod_complex_comments.0,
            mod_comments_code: mod_comments_code.0,
            mod_comments_cdn: mod_comments_cdn.0,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(transparent)]
pub struct RemovalReasonKey(String);

impl std::fmt::Display for RemovalReasonKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl RemovalReasonKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

super::impl_sql_fwd!(RemovalReasonKey as String);

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct RemovalReasonsMap {
    map: HashMap<RemovalReasonKey, String>,
}

impl Borrow<str> for RemovalReasonKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl RemovalReasonsMap {
    pub fn get(&self, reason: &RemovalReasonKey) -> Option<&str> {
        self.map.get(reason).map(|v| v.as_str())
    }

    pub fn get_or_default(&self, reason: &RemovalReasonKey) -> Option<&str> {
        match self.get(reason) {
            Some(r) => Some(r),
            None => self.map.get("#default").map(|v| v.as_str()),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, mapped: impl Into<String>) {
        self.map.insert(RemovalReasonKey(key.into()), mapped.into());
    }

    pub fn with(mut self, key: impl Into<String>, mapped: impl Into<String>) -> Self {
        self.insert(key, mapped);
        self
    }
}

impl<'a> IntoIterator for &'a RemovalReasonsMap {
    type Item = (&'a RemovalReasonKey, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, RemovalReasonKey, String>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.map).into_iter()
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StaffReplyModule {
    pub enabled: bool,
    pub flair_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css_class: Option<String>,
    pub ignore_post_title_contains: Vec<String>,
}

impl StaffReplyModule {
    pub fn is_staff(&self, template_id: Option<&str>, css_class: Option<&str>) -> bool {
        template_id.is_some_and(|id| id == self.flair_id)
            || self.css_class.as_ref().map(|v| v.as_str()) == css_class
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusModule {
    pub enabled: bool,
    pub min_impact: statuspage::incident::IncidentImpact,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticky: Option<StatusStickyConfig>,
    pub distinguish: bool,
}

impl Default for StatusModule {
    fn default() -> Self {
        Self {
            enabled: false,
            min_impact: statuspage::incident::IncidentImpact::Critical,
            sticky: None,
            distinguish: false,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusStickyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_sticky: Option<String>,
    pub comment_threshold: u64,
    pub delay_minor_mins: u64,
    pub delay_major_mins: u64,
    pub min_impact: Option<statuspage::incident::IncidentImpact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_for: Option<Vec<String>>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AiSlopModule {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modmail_to: Option<String>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComplexCommentRule {
    pub name: String,
    pub reason: String,
    pub link_title: Vec<String>,
    pub comment: Vec<String>,
    #[serde(default)]
    pub ignore_flairs: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComplexCommentsModule {
    pub enabled: bool,
    #[serde(default)]
    pub items: Vec<ComplexCommentRule>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RelatedTitleModule {
    pub enabled: bool,
    pub reason: RemovalReasonKey,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScamsModule {
    pub enabled: bool,
    #[serde(default)]
    pub search_modqueue: bool,
}

macro_rules! make_simple_module {
    ($($name:ident),* $(,)?) => {
        $(

            #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
            pub struct $name {
                pub enabled: bool,
            }
        )*
    };
}

make_simple_module!(CommentsCdnModule, CommentsCodeModule);

super::make_newtype_id!(ReplyTemplateId, ScamId);

#[derive(Debug)]
pub struct ReplyTemplate {
    pub id: ReplyTemplateId,
    pub name: String,
    pub content: String,
}

impl ReplyTemplate {
    pub(crate) fn from_row(row: Row) -> DbResult<Self> {
        let id = row.get("id");
        let name = row.get("name");
        let content = row.get("content");

        Ok(Self { id, name, content })
    }
}

#[derive(Debug)]
pub struct Scam {
    pub id: ScamId,
    pub name: String,
    pub enabled: bool,
    pub self_post: bool,
    pub remove: bool,
    pub report: bool,

    pub ocr: Option<Matchers>,
    pub title: Option<Matchers>,
    pub body: Option<Matchers>,
    pub title_or_body: Option<Matchers>,

    pub reason: Option<RemovalReasonKey>,
    pub template: Option<ReplyTemplateId>,
}

impl Scam {
    pub(crate) fn from_row(row: Row) -> DbResult<Self> {
        let id = row.get("id");
        let name = row.get("name");
        let enabled = row.get("enabled");
        let self_post = row.get("self_post");
        let remove = row.get("remove");
        let report = row.get("report");
        let ocr: Option<Json<Matchers>> = row.get("ocr");
        let title: Option<Json<Matchers>> = row.get("title");
        let body: Option<Json<Matchers>> = row.get("body");
        let title_or_body: Option<Json<Matchers>> = row.get("title_or_body");
        let reason = row.get("reason");
        let template = row.get("template");

        Ok(Self {
            id,
            name,
            enabled,
            self_post,
            remove,
            report,
            ocr: ocr.map(|v| v.0),
            title: title.map(|v| v.0),
            body: body.map(|v| v.0),
            title_or_body: title_or_body.map(|v| v.0),
            reason,
            template,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{client::PgClientBuilder, errors::DbResult, repos::subreddits::SubredditsRepo};

    #[tokio::test]
    async fn dothething() -> DbResult<()> {
        let pg = PgClientBuilder::new("postgres://postgres:postgres@localhost/mlapibot")
            .connect()
            .await?;

        let subs = pg.fetch_all_subreddits().await?;

        for sub in subs {
            println!("/r/{}", sub.name);

            let templates = pg.get_subreddit_templates(&sub.id).await?;

            for t in templates {
                println!("  # {t:?}");
            }

            let scams = pg.get_subreddit_scams(&sub.id).await?;

            for s in scams {
                println!("  > {s:?}");
            }
        }

        Ok(())
    }
}
