use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio_postgres::{
    AsyncMessage, Row, Statement, ToStatement, Transaction,
    types::{FromSqlOwned, ToSql},
};

use crate::{
    errors::{DbError, DbResult},
    migrations::apply_migrations,
};

#[derive(Debug)]
pub struct PgNotification {
    pub channel: String,
    pub payload: String,
}

pub struct PgClientBuilder {
    url: String,
    do_migrate: bool,
    listen_chnl: Option<Sender<PgNotification>>,
}

impl PgClientBuilder {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            do_migrate: true,
            listen_chnl: None,
        }
    }

    pub fn no_migrate(mut self) -> Self {
        self.do_migrate = false;
        self
    }

    pub fn listen(mut self, tx: Sender<PgNotification>) -> Self {
        self.listen_chnl = Some(tx);
        self
    }

    pub async fn connect(self) -> DbResult<PgClient> {
        let Self {
            url,
            do_migrate,
            listen_chnl,
        } = self;

        let (client, mut conn) = tokio_postgres::connect(&url, tokio_postgres::NoTls).await?;

        let should_listen = match listen_chnl {
            None => {
                tokio::spawn(async move {
                    if let Err(e) = conn.await {
                        eprintln!("DB background connect failed: {e}");
                    }
                });

                false
            }
            Some(tx) => {
                tokio::spawn(async move {
                    let mut stream = futures::stream::poll_fn(move |cx| conn.poll_message(cx));

                    while let Some(Ok(msg)) = stream.next().await {
                        match msg {
                            AsyncMessage::Notification(n) => {
                                let n = PgNotification {
                                    channel: n.channel().to_owned(),
                                    payload: n.payload().to_owned(),
                                };

                                let Ok(()) = tx.send(n).await else {
                                    break;
                                };
                            }
                            AsyncMessage::Notice(n) => {
                                eprintln!("[db] async message: {n:?}");
                            }
                            other => {
                                eprintln!("unrecognised async message: {other:?}");
                            }
                        }
                    }
                });

                true
            }
        };

        // Preparing the statement will check the table definitions to make sure they're valid,
        // but the migrations may not have been ran yet and the migrations want the `PgClient`
        // for the util methods.
        // So use a dummy statement for the migrations, then make the actual ones after.
        let null_stmt = client.prepare("SELECT NULL").await?;
        let mut this = PgClient {
            client,
            stmt_is_monitored: null_stmt,
        };

        if do_migrate {
            apply_migrations(&mut this).await?;
        }

        this.stmt_is_monitored = this
            .client
            .prepare(crate::repos::monitor::IS_MONITORED_QUERY)
            .await?;

        if should_listen {
            this.execute("LISTEN mlapibot;", &[]).await?;
        }

        Ok(this)
    }
}

pub struct PgClient {
    client: tokio_postgres::Client,
    /// Since we are going to do this a lot to check whether a post/comment is new,
    /// we cache the query.
    pub(crate) stmt_is_monitored: Statement,
}

impl PgClient {
    pub async fn danger_delete_all_data(&self) -> DbResult<()> {
        self.client
            .batch_execute(
                r#"
                DELETE FROM monitored;
                DELETE FROM staff_replies;
                DELETE FROM staff_reply_threads;
                DELETE FROM incident_sub_posts;
                DELETE FROM status_incidents;
                DELETE FROM subreddit_mods;
                DELETE FROM subreddit_scam_rules;
                DELETE FROM subreddits;
                DELETE FROM users;
                "#,
            )
            .await?;

        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn prepare(&self, query: &str) -> DbResult<Statement> {
        self.client.prepare(query).await.map_err(DbError::from)
    }

    pub(crate) async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> DbResult<u64>
    where
        T: ?Sized + ToStatement,
    {
        self.client
            .execute(statement, params)
            .await
            .map_err(DbError::from)
    }

    #[allow(unused)]
    pub(crate) async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> DbResult<Option<Row>>
    where
        T: ToStatement + ?Sized,
    {
        self.client
            .query_opt(statement, params)
            .await
            .map_err(DbError::from)
    }

    pub(crate) async fn query_opt_map<T, F, R>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
        map: F,
    ) -> DbResult<Option<R>>
    where
        F: FnOnce(Row) -> DbResult<R>,
        T: ToStatement + ?Sized,
    {
        match self.client.query_opt(statement, params).await {
            Ok(Some(item)) => map(item).map(Some),
            Ok(None) => Ok(None),
            Err(err) => Err(DbError::from(err)),
        }
    }

    pub(crate) async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> DbResult<Row>
    where
        T: ToStatement + ?Sized,
    {
        self.client
            .query_one(statement, params)
            .await
            .map_err(DbError::from)
    }

    pub(crate) async fn query_scalar<T, R>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> DbResult<Vec<R>>
    where
        T: std::fmt::Debug + ToStatement + ?Sized,
        R: FromSqlOwned,
    {
        self.client
            .query_scalar(statement, params)
            .await
            .map_err(DbError::from)
    }

    pub(crate) async fn query_one_scalar<T, R>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> DbResult<R>
    where
        T: std::fmt::Debug + ToStatement + ?Sized,
        R: FromSqlOwned,
    {
        self.client
            .query_one_scalar(statement, params)
            .await
            .map_err(DbError::from)
    }

    pub(crate) async fn query_one_map<T, F, R>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
        map: F,
    ) -> DbResult<R>
    where
        T: ToStatement + ?Sized,
        F: FnOnce(Row) -> DbResult<R>,
    {
        self.query_one(statement, params).await.and_then(map)
    }

    pub(crate) async fn query_map<T, F, R>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
        mut map: F,
    ) -> DbResult<Vec<R>>
    where
        T: ToStatement + ?Sized,
        F: FnMut(Row) -> DbResult<R>,
    {
        let items = self.client.query(statement, params).await?;
        let mut mapped = Vec::with_capacity(items.len());

        for item in items {
            let next = map(item)?;
            mapped.push(next);
        }

        Ok(mapped)
    }

    pub(crate) async fn transaction(&mut self) -> DbResult<Transaction<'_>> {
        self.client.transaction().await.map_err(DbError::from)
    }
}
