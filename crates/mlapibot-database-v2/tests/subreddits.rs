use chrono::SubsecRound;
use pretty_assertions::assert_eq;

use mlapibot_database_v2::{
    client::{PgClient, PgClientBuilder},
    errors::DbResult,
    repos::subreddits::*,
};

async fn make_db() -> PgClient {
    let db = PgClientBuilder::new("postgres://postgres:postgres@localhost/mlapibotest")
        .connect()
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

    let now = chrono::Utc::now().trunc_subsecs(1);

    let mods_ids = vec!["user01", "user02", "user33"];

    let subreddit = Subreddit {
        id: "sub123".into(),
        name: "subreddit".into(),
        enabled: true,
        last_sync: now,
        seq_num: 1,
        mod_json_schema: 0,
        removal_reasons: RemovalReasonsMap::default().with("#repost", "abc-rule-123"),
        mod_scams: ScamsModule {
            enabled: true,
            search_modqueue: true,
        },
        mod_ai_slop: AiSlopModule {
            enabled: false,
            report: true,
            modmail_to: Some("sub456".into()),
        },
        mod_staff_reply: StaffReplyModule {
            enabled: false,
            flair_id: String::new(),
            css_class: None,
            ignore_post_title_contains: Vec::new(),
        },
        mod_status: StatusModule {
            enabled: true,
            min_impact: statuspage::incident::IncidentImpact::Critical,
            sticky: None,
            distinguish: true,
            flair_id: Some("abc-flair-123".into()),
        },
        mod_related_title: RelatedTitleModule {
            enabled: true,
            reason: RemovalReasonKey::new("#repost"),
            check_img_posts: true,
            auto_add_remove_text: Some("vague title".into()),
        },
        mod_complex_comments: ComplexCommentsModule {
            enabled: true,
            items: Vec::new(),
        },
        mod_comments_code: CommentsCodeModule { enabled: false },
        mod_comments_cdn: CommentsCdnModule { enabled: false },
    };

    db.create_subreddit(&subreddit).await?;

    db.set_subreddit_moderators(&subreddit.id, &mods_ids)
        .await?;

    let mods = db.get_subreddit_moderators(&subreddit.id).await?;
    assert_eq!(mods, mods_ids);

    let mut subs = db.fetch_all_subreddits().await?;

    let fromdb = subs[0].last_sync.trunc_subsecs(1);
    assert_eq!(fromdb, subreddit.last_sync);
    subs[0].last_sync = subreddit.last_sync;
    assert_eq!(subs[0], subreddit);

    let words = db.get_vague_words(&subreddit.id).await?;
    assert_eq!(words.len(), 0);

    db.add_vague_words(&subreddit.id, &["hello", "world", "wowza"])
        .await?;

    let words = db.get_vague_words(&subreddit.id).await?;
    assert_eq!(words.len(), 3);
    assert!(words.contains("hello"));
    assert!(words.contains("world"));
    assert!(words.contains("wowza"));

    db.add_vague_words(&subreddit.id, &["hello", "extra"])
        .await?;

    let words = db.get_vague_words(&subreddit.id).await?;
    assert_eq!(words.len(), 4);
    assert!(words.contains("hello"));
    assert!(words.contains("world"));
    assert!(words.contains("wowza"));
    assert!(words.contains("extra"));

    Ok(())
}
