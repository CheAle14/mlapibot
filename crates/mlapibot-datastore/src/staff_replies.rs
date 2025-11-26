use rusqlite::OptionalExtension;

use crate::{DateTimeUtc, QueryCollect};

#[derive(Debug, Default)]
pub struct StaffReply {
    pub comment_id: String,
    pub post_id: String,
    pub author_name: String,
    pub content: String,
    pub last_updated: DateTimeUtc,
    pub created_at: DateTimeUtc,
}

impl StaffReply {
    pub fn num_newlines(&self) -> usize {
        self.content.chars().filter(|c| *c == '\n').count()
    }

    pub fn is_outdated(&self, now: DateTimeUtc) -> bool {
        let since_created = now.signed_duration_since(self.created_at).abs();
        let since_updated = now.signed_duration_since(self.last_updated).abs();

        // e.g.:
        // if made 5 minutes ago, check if last update was 5 minutes ago
        // if made 30 minutes ago, check if last update was 30 minutes ago
        // this causes linearly increasing gaps between updates, since most
        // comments are likely edited shortly after they are made, but rarely
        // after that.
        since_updated < since_created
    }
}

pub struct StaffReplyThread {
    pub post_id: String,
    pub our_comment_id: String,
}

impl super::MlapiDb {
    pub fn insert_staff_reply(
        &self,
        comment_id: &str,
        post_id: &str,
        author_name: &str,
        content: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute("INSERT INTO StaffReplies (CommentId, PostId, AuthorName, Content) VALUES (?1, ?2, ?3, ?4)",
            (comment_id, post_id, author_name, content))?;

        Ok(())
    }

    pub fn get_staff_replies_in(&self, post_id: &str) -> rusqlite::Result<Vec<StaffReply>> {
        let mut stmt = self.conn.prepare(
            "SELECT CommentId, PostId, AuthorName, Content, LastUpdated, CreatedAt
            FROM StaffReplies
            WHERE PostId=?1
            ORDER BY CreatedAt ASC
            ",
        )?;

        stmt.query_collect((post_id,), |row| {
            let comment_id = row.get(0)?;
            let post_id = row.get(1)?;
            let author_name = row.get(2)?;
            let content = row.get(3)?;
            let last_updated = row.get(4)?;
            let created_at = row.get(5)?;

            Ok(StaffReply {
                comment_id,
                post_id,
                author_name,
                content,
                last_updated,
                created_at,
            })
        })
    }

    pub fn insert_staff_reply_thread(
        &self,
        post_id: &str,
        our_comment_id: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO StaffReplyThreads (PostId, OurCommentId) VALUES (?1, ?2)",
            (post_id, our_comment_id),
        )?;

        Ok(())
    }

    pub fn get_staff_reply_thread(
        &self,
        post_id: &str,
    ) -> rusqlite::Result<Option<StaffReplyThread>> {
        self.conn
            .query_row(
                "SELECT PostId, OurCommentId FROM StaffReplyThreads WHERE PostId=?1",
                (post_id,),
                |row| {
                    let post_id = row.get(0)?;
                    let our_comment_id = row.get(1)?;

                    Ok(StaffReplyThread {
                        post_id,
                        our_comment_id,
                    })
                },
            )
            .optional()
    }
}
