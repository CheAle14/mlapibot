use tokio_postgres::{
    Row, Statement, ToStatement, Transaction,
    types::{FromSqlOwned, ToSql},
};

use crate::{
    errors::{DbError, DbResult},
    migrations::apply_migrations,
};

pub struct PgClient {
    client: tokio_postgres::Client,
    // Since we are going to do this a lot to check whether a post/comment is new,
    // we cache the query.
    // pub(crate) stmt_is_monitored: Statement,
}

impl PgClient {
    pub async fn connect(url: &str) -> DbResult<Self> {
        let (client, conn) = tokio_postgres::connect(url, tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("DB background connect failed: {e}");
            }
        });

        // let stmt_is_monitored = client
        //     .prepare(crate::repos::monitor::IS_MONITORED_QUERY)
        //     .await?;

        Ok(Self {
            client,
            //stmt_is_monitored,
        })
    }

    pub async fn danger_delete_all_data(&self) -> DbResult<()> {
        self.client
            .batch_execute(
                r#"
                DELETE FROM monitored;
                DELETE FROM staff_replies;
                DELETE FROM staff_reply_threads;
                DELETE FROM incident_sub_posts;
                DELETE FROM status_incidents;
                "#,
            )
            .await?;

        Ok(())
    }

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
