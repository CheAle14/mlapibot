use chrono::{SubsecRound, Utc};
use mlapibot_database_v2::{client::PgClient, errors::DbResult, repos::subreddits::*};

async fn make_db() -> PgClient {
    let db = PgClient::connect("postgres://postgres:postgres@localhost/mlapibotest", true)
        .await
        .expect("can connect");

    db.danger_delete_all_data()
        .await
        .expect("can delete everything in DB");

    db
}

#[tokio::test]
async fn insert_and_fetch_subreddits() -> DbResult<()> {
    let mut db = make_db().await;

    let now = chrono::Utc::now();

    let mods_ids = vec!["user01", "user02", "user33"];

    let subreddit = Subreddit {
        id: "sub123".into(),
        name: "subreddit".into(),
        last_sync: now,
        seq_num: 1,
        mod_json_schema: 0,
        mod_scams: ScamsModule { enabled: true },
        mod_ai_slop: AiSlopModule { enabled: false },
        mod_staff_reply: StaffReplyModule {
            enabled: false,
            flair_id: String::new(),
            css_class: None,
        },
        mod_status: StatusModule {
            enabled: true,
            min_impact: statuspage::incident::IncidentImpact::Critical,
            sticky: None,
            distinguish: true,
        },
        mod_related_title: RelatedTitleModule { enabled: true },
    };

    db.create_subreddit(&subreddit).await?;

    db.set_subreddit_moderators(&subreddit.id, &mods_ids)
        .await?;

    let mods = db.get_subreddit_moderators(&subreddit.id).await?;
    assert_eq!(mods, mods_ids);

    let subs = db.fetch_all_subreddits().await?;
    assert_eq!(subs, vec![subreddit]);

    Ok(())
}
