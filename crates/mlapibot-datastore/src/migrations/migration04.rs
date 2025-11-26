pub struct StaffReplyComment;

impl super::Migration for StaffReplyComment {
    fn apply(&self, db: &mut crate::MlapiDb) -> rusqlite::Result<()> {
        db.conn.execute_batch(
            "
            CREATE TABLE StaffReplies (
                CommentId       TEXT NOT NULL PRIMARY KEY,
                PostId          TEXT NOT NULL,
                LastUpdated     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
                AuthorName      TEXT NOT NULL,
                Content         TEXT NOT NULL,
                CreatedAt       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
            );

            CREATE INDEX StaffRepliesPostId ON StaffReplies (PostId);

            CREATE TABLE StaffReplyThreads (
                PostId          TEXT NOT NULL PRIMARY KEY,
                OurCommentId    TEXT NOT NULL
            );

        ",
        )?;

        Ok(())
    }
}
