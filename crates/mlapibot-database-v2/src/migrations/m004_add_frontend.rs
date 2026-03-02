pub struct AddFrontend;

impl super::Migration for AddFrontend {
    async fn apply(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            CREATE TABLE subreddits (
                id              TEXT        PRIMARY KEY NOT NULL,
                name            TEXT        NOT NULL,
                enabled         BOOLEAN     NOT NULL DEFAULT TRUE,
                last_sync       TIMESTAMPTZ NOT NULL,

                seq_num         INTEGER     NOT NULL DEFAULT 0,

                removal_reasons     JSONB       NOT NULL,
                mod_json_schema     INTEGER     NOT NULL DEFAULT 0,
                mod_scams           JSONB       NOT NULL,
                mod_ai_slop         JSONB       NOT NULL,
                mod_staff_reply     JSONB       NOT NULL,
                mod_status          JSONB       NOT NULL,
                mod_related_title   JSONB       NOT NULL,
                mod_complex_comments JSONB      NOT NULL,
                mod_comments_code   JSONB       NOT NULL,
                mod_comments_cdn    JSONB       NOT NULL
            );

            CREATE TABLE subreddit_mods (
                subreddit_id    TEXT        REFERENCES subreddits(id) ON DELETE CASCADE NOT NULL ,
                username       TEXT        NOT NULL,

                PRIMARY KEY (subreddit_id, username)
            );

            CREATE TABLE subreddit_templates (
                id              SERIAL      PRIMARY KEY NOT NULL,
                subreddit_id    TEXT        REFERENCES subreddits(id) ON DELETE CASCADE NOT NULL,
                name            TEXT        NOT NULL,
                content         TEXT        NOT NULL
            );

            CREATE TABLE subreddit_scam_rules (
                id              SERIAL      PRIMARY KEY NOT NULL,
                subreddit_id    TEXT        REFERENCES subreddits(id) ON DELETE CASCADE NOT NULL,
                name            TEXT        NOT NULL,

                ocr             JSONB       NULL,
                title           JSONB       NULL,
                body            JSONB       NULL,
                title_or_body   JSONB       NULL,

                reason          TEXT        NULL,
                template        INTEGER     REFERENCES subreddit_templates(id) ON DELETE SET NULL,

                enabled         BOOLEAN     NOT NULL,
                self_post       BOOLEAN     NOT NULL,
                remove          BOOLEAN     NOT NULL,
                report          BOOLEAN     NOT NULL
            );

            CREATE TABLE users (
                id              TEXT        PRIMARY KEY NOT NULL,
                name            TEXT        NOT NULL,

                admin           BOOL        NOT NULL DEFAULT false,
                cookie          TEXT        NULL,
                last_sync       TIMESTAMPTZ NULL DEFAULT CURRENT_TIMESTAMP
            );
        ",
        )
        .await?;

        Ok(())
    }

    async fn undo(conn: &tokio_postgres::Transaction<'_>) -> crate::errors::DbResult<()> {
        conn.batch_execute(
            "
            DROP TABLE users;
            DROP TABLE subreddit_scam_rules;
            DROP TABLE subreddit_templates;
            DROP TABLE subreddit_mods;
            DROP TABLE subreddits;

            ",
        )
        .await?;

        Ok(())
    }
}
