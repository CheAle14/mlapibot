use mlapibot_database_v2::{
    client::PgClient,
    errors::DbResult,
    repos::monitor::{MonitorRepo, MonitorState},
};

async fn make_db() -> PgClient {
    let db = PgClient::connect("postgres://postgres:postgres@localhost/mlapibot", true)
        .await
        .expect("can connect");

    db.danger_delete_all_data()
        .await
        .expect("can delete everything in DB");

    db
}

#[tokio::test]
async fn can_insert_and_test_monitored() -> DbResult<()> {
    let db = make_db().await;

    assert_eq!(db.has_seen_item("post123").await?, false);

    db.mark_item_seen("post123", "subreddit0").await?;
    db.mark_item_seen("other123", "subreddit0").await?;

    assert_eq!(db.has_seen_item("post123").await?, true);

    assert_eq!(
        db.get_item_monitor_state("post123").await?,
        MonitorState::Seen
    );

    db.update_item_monitor_state(
        "post123",
        MonitorState::Acted {
            analyzer: String::from("scam/123"),
            reply_fullname: None,
            reported: true,
            removed: false,
            mistaken: false,
        },
    )
    .await?;

    assert_eq!(
        db.get_item_monitor_state("post123").await?,
        MonitorState::Acted {
            analyzer: String::from("scam/123"),
            reply_fullname: None,
            reported: true,
            removed: false,
            mistaken: false,
        }
    );

    db.set_item_mistaken("post123", true).await?;

    assert_eq!(
        db.get_item_monitor_state("post123").await?,
        MonitorState::Acted {
            analyzer: String::from("scam/123"),
            reply_fullname: None,
            reported: true,
            removed: false,
            mistaken: true,
        }
    );

    assert_eq!(
        db.get_item_monitor_state("other123").await?,
        MonitorState::Seen,
        "shouldn't affect other posts"
    );

    db.update_item_monitor_state("post123", MonitorState::Seen)
        .await?;

    db.update_item_monitor_state("other123", MonitorState::Ignored)
        .await?;

    assert_eq!(
        db.get_item_monitor_state("other123").await?,
        MonitorState::Ignored,
    );

    assert_eq!(
        db.get_item_monitor_state("post123").await?,
        MonitorState::Seen,
    );

    Ok(())
}

#[tokio::test]
async fn can_delete_monitored() -> DbResult<()> {
    let db = make_db().await;

    db.mark_item_seen("post123", "subreddit0").await?;
    db.mark_item_seen("other123", "subreddit0").await?;

    assert_eq!(
        db.get_item_monitor_state("post123").await?,
        MonitorState::Seen
    );

    assert_eq!(db.delete_monitored("post123").await?, true);

    assert!(db.get_item_monitor_state("post123").await.is_err(),);

    assert_eq!(
        db.get_item_monitor_state("other123").await?,
        MonitorState::Seen
    );

    Ok(())
}
