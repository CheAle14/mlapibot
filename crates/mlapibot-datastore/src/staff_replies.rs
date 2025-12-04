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

    pub fn get_next_update(&self) -> DateTimeUtc {
        let between_create_and_update = self
            .last_updated
            .signed_duration_since(self.created_at)
            .abs();

        self.last_updated + between_create_and_update
    }

    pub fn is_outdated(&self, now: DateTimeUtc) -> bool {
        self.get_next_update() <= now
    }
}

#[derive(Debug)]
pub struct StaffReplyThread {
    pub subreddit: String,
    pub post_id: String,
    pub our_comment_id: String,
    pub created_at: DateTimeUtc,
}

pub enum FindBy {
    PostId,
    OurCommentId,
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

    pub fn update_staff_reply(&self, reply: &StaffReply) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE StaffReplies SET AuthorName=?1, Content=?2, LastUpdated=?3 WHERE CommentId=?4",
            (
                &reply.author_name,
                &reply.content,
                reply.last_updated,
                &reply.comment_id,
            ),
        )?;

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
        subreddit: &str,
        post_id: &str,
        our_comment_id: &str,
        created_at: DateTimeUtc,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO StaffReplyThreads (Subreddit, PostId, OurCommentId, CreatedAt) VALUES (?1, ?2, ?3, ?4)",
            (subreddit, post_id, our_comment_id, created_at),
        )?;

        Ok(())
    }

    pub fn get_staff_reply_thread(
        &self,
        find_by: FindBy,
        id: &str,
    ) -> rusqlite::Result<Option<StaffReplyThread>> {
        let query = match find_by {
            FindBy::PostId => {
                "SELECT Subreddit, PostId, OurCommentId, CreatedAt FROM StaffReplyThreads WHERE PostId=?1"
            }
            FindBy::OurCommentId => {
                "SELECT Subreddit, PostId, OurCommentId, CreatedAt FROM StaffReplyThreads WHERE OurCommentId=?1"
            }
        };

        self.conn
            .query_row(query, (id,), |row| {
                let subreddit = row.get(0)?;
                let post_id = row.get(1)?;
                let our_comment_id = row.get(2)?;
                let created_at = row.get(3)?;

                Ok(StaffReplyThread {
                    subreddit,
                    post_id,
                    our_comment_id,
                    created_at,
                })
            })
            .optional()
    }

    pub fn get_staff_reply_threads_in(
        &self,
        subreddit: &str,
        after: DateTimeUtc,
    ) -> rusqlite::Result<Vec<StaffReplyThread>> {
        let mut stmt = self.conn.prepare(
            "SELECT Subreddit, PostId, OurCommentId, CreatedAt
            FROM StaffReplyThreads
            WHERE Subreddit=?1 AND CreatedAt >= ?2
            ",
        )?;

        stmt.query_collect((subreddit, after), |row| {
            let subreddit = row.get(0)?;
            let post_id = row.get(1)?;
            let our_comment_id = row.get(2)?;
            let created_at = row.get(3)?;

            Ok(StaffReplyThread {
                subreddit,
                post_id,
                our_comment_id,
                created_at,
            })
        })
    }
}
