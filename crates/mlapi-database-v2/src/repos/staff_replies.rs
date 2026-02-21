use crate::{DateTimeUtc, client::PgClient};

#[derive(Debug, Default, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct StaffReplyThread {
    pub subreddit: String,
    pub post_id: String,
    pub our_comment_id: String,
    pub created_at: DateTimeUtc,
    pub hash: String,
    pub suffix: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct SubredditStaffReplyThread {
    pub subreddit: String,
    pub post_id: String,
}

pub enum FindBy {
    PostId,
    OurCommentId,
}

pub trait StaffReplyRepo {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn insert_staff_reply(
        &self,
        comment_id: &str,
        post_id: &str,
        author_name: &str,
        content: &str,
    ) -> Result<(), Self::Error>;

    async fn update_staff_reply_content(
        &self,
        comment_id: &str,
        content: &str,
    ) -> Result<(), Self::Error>;

    async fn update_staff_reply_thread_suffix(
        &self,
        post_id: &str,
        suffix: Option<&str>,
    ) -> Result<(), Self::Error>;

    async fn update_staff_reply_thread(&self, post_id: &str, hash: &str)
    -> Result<(), Self::Error>;

    async fn get_staff_replies_in(&self, post_id: &str) -> Result<Vec<StaffReply>, Self::Error>;

    async fn insert_staff_reply_thread(
        &self,
        subreddit: &str,
        post_id: &str,
        our_comment_id: &str,
        hash: &str,
    ) -> Result<(), Self::Error>;

    async fn get_staff_reply_thread(
        &self,
        find_by: FindBy,
        id: &str,
    ) -> Result<Option<StaffReplyThread>, Self::Error>;

    async fn get_staff_reply_threads_in(
        &self,
        subreddit: &str,
        after: DateTimeUtc,
    ) -> Result<Vec<SubredditStaffReplyThread>, Self::Error>;
}

impl StaffReplyRepo for PgClient {
    type Error = crate::errors::DbError;

    async fn insert_staff_reply(
        &self,
        comment_id: &str,
        post_id: &str,
        author_name: &str,
        content: &str,
    ) -> Result<(), Self::Error> {
        self.execute("INSERT INTO staff_replies (comment_id, post_id, author_name, content) VALUES ($1, $2, $3, $4)",
            &[&comment_id, &post_id, &author_name, &content]).await?;

        Ok(())
    }

    async fn update_staff_reply_content(
        &self,
        comment_id: &str,
        content: &str,
    ) -> Result<(), Self::Error> {
        self.execute(
            r"
            UPDATE staff_replies SET
                last_updated=CURRENT_TIMESTAMP,
                content=$2
            WHERE comment_id=$1
            ",
            &[&comment_id, &content],
        )
        .await?;

        Ok(())
    }

    async fn update_staff_reply_thread_suffix(
        &self,
        post_id: &str,
        suffix: Option<&str>,
    ) -> Result<(), Self::Error> {
        self.execute(
            r"
            UPDATE staff_reply_threads SET
                suffix=$2
            WHERE post_id=$1
            ",
            &[&post_id, &suffix],
        )
        .await?;

        Ok(())
    }

    async fn update_staff_reply_thread(
        &self,
        post_id: &str,
        hash: &str,
    ) -> Result<(), Self::Error> {
        self.execute(
            r"
            UPDATE staff_reply_threads SET
                hash=$2
            WHERE post_id=$1
            ",
            &[&post_id, &hash],
        )
        .await?;

        Ok(())
    }

    async fn get_staff_replies_in(&self, post_id: &str) -> Result<Vec<StaffReply>, Self::Error> {
        self.query_map(
            "
            SELECT comment_id, author_name, content, last_updated, created_at
            FROM staff_replies
            WHERE post_id=$1",
            &[&post_id],
            |r| {
                Ok(StaffReply {
                    comment_id: r.get(0),
                    post_id: post_id.to_owned(),
                    author_name: r.get(1),
                    content: r.get(2),
                    last_updated: r.get(3),
                    created_at: r.get(4),
                })
            },
        )
        .await
    }

    async fn insert_staff_reply_thread(
        &self,
        subreddit: &str,
        post_id: &str,
        our_comment_id: &str,
        hash: &str,
    ) -> Result<(), Self::Error> {
        self.execute("INSERT INTO staff_reply_threads (post_id, our_comment_id, subreddit, hash) VALUES ($1, $2, $3, $4)",
            &[&post_id, &our_comment_id, &subreddit, &hash]).await?;
        Ok(())
    }

    async fn get_staff_reply_thread(
        &self,
        find_by: FindBy,
        id: &str,
    ) -> Result<Option<StaffReplyThread>, Self::Error> {
        let stmt = match find_by {
            FindBy::PostId => {
                "SELECT post_id, our_comment_id, subreddit, created_at, hash, suffix
                FROM staff_reply_threads
                WHERE post_id=$1"
            }
            FindBy::OurCommentId => {
                "SELECT post_id, our_comment_id, subreddit, created_at, hash, suffix
                FROM staff_reply_threads
                WHERE our_comment_id=$1"
            }
        };

        self.query_opt_map(stmt, &[&id], |r| {
            Ok(StaffReplyThread {
                post_id: r.get(0),
                our_comment_id: r.get(1),
                subreddit: r.get(2),
                created_at: r.get(3),
                hash: r.get(4),
                suffix: r.get(5),
            })
        })
        .await
    }

    async fn get_staff_reply_threads_in(
        &self,
        subreddit: &str,
        after: DateTimeUtc,
    ) -> Result<Vec<SubredditStaffReplyThread>, Self::Error> {
        self.query_map(
            "
            SELECT post_id
            FROM staff_reply_threads
            WHERE subreddit=$1 AND created_at >= $2",
            &[&subreddit, &after],
            |r| {
                Ok(SubredditStaffReplyThread {
                    subreddit: subreddit.to_owned(),
                    post_id: r.get(0),
                })
            },
        )
        .await
    }
}
