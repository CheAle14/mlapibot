use mlapi_database_v2::{
    client::PgClient, errors::DbResult, migrations::apply_migrations, repos::staff_replies::*,
};

async fn make_db() -> PgClient {
    let mut db = PgClient::connect("postgres://postgres:postgres@localhost/mlapibot")
        .await
        .expect("can connect");

    apply_migrations(&mut db)
        .await
        .expect("can apply migrations");

    db.danger_delete_all_data()
        .await
        .expect("can delete everything in DB");

    db
}

#[tokio::test]
async fn can_insert_and_list_staff_replies() -> DbResult<()> {
    let db = make_db().await;

    let now = chrono::Utc::now();

    db.insert_staff_reply_thread("sub0123", "post123", "comment032", "abchash")
        .await?;

    db.insert_staff_reply("comment112", "post123", "Hello", "some text")
        .await?;
    db.insert_staff_reply("comment113", "post123", "World", "other types")
        .await?;

    let threads = db.get_staff_reply_threads_in("sub0123", now).await?;

    assert_eq!(
        threads,
        vec![SubredditStaffReplyThread {
            subreddit: "sub0123".into(),
            post_id: "post123".into()
        }]
    );

    let comments = db.get_staff_replies_in("post123").await?;
    assert_eq!(comments.len(), 2);

    Ok(())
}

#[tokio::test]
async fn can_update_staff_replies() -> DbResult<()> {
    let db = make_db().await;

    let now = chrono::Utc::now();

    db.insert_staff_reply_thread("sub0123", "post123", "comment032", "abchash")
        .await?;

    db.insert_staff_reply("comment112", "post123", "Hello", "some text")
        .await?;

    db.update_staff_reply_thread_suffix("post123", Some("hello world"))
        .await?;

    db.update_staff_reply_content("comment112", "another text")
        .await?;

    let mut thread = db
        .get_staff_reply_thread(FindBy::PostId, "post123")
        .await?
        .expect("can find thread");

    assert!(
        thread
            .created_at
            .signed_duration_since(now)
            .as_seconds_f32()
            < 5.0
    );
    thread.created_at = now; // so we can use assert_eq

    assert_eq!(
        thread,
        StaffReplyThread {
            subreddit: "sub0123".into(),
            post_id: "post123".into(),
            our_comment_id: "comment032".into(),
            created_at: now,
            hash: "abchash".into(),
            suffix: Some("hello world".into())
        }
    );

    Ok(())
}
