pub struct InitIncidents;

impl super::Migration for InitIncidents {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            CREATE TABLE status_incidents (
                incident_id         TEXT        PRIMARY KEY,
                updated_at          TIMESTAMPTZ NULL,
                resolved_at         TIMESTAMPTZ NULL,

                live_fullname            TEXT        NOT NULL
            );

            CREATE TYPE IncidentStickyState AS ENUM ('never', 'currently', 'undone');

            CREATE TABLE incident_sub_posts (
                subreddit           TEXT        NOT NULL,
                incident_id         TEXT        NOT NULL,
                fullname            TEXT        UNIQUE NOT NULL,
                unstickied_at       TIMESTAMPTZ NULL,
                sticky_state        IncidentStickyState NOT NULL DEFAULT 'never',
                prior_sticky        TEXT    NULL,

                PRIMARY KEY(subreddit, incident_id)
            );
        ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            DROP TABLE incident_sub_posts;
            DROP TYPE IncidentStickyState;
            DROP TABLE incident_live_threads;
            ",
        )
        .await?;

        Ok(())
    }
}
