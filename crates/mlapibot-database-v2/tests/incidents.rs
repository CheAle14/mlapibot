use chrono::{SubsecRound, Utc};
use mlapibot_database_v2::{
    client::PgClient, errors::DbResult, migrations::apply_migrations, repos::incidents::*,
};

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
async fn status_incidents_lifecycle() -> DbResult<()> {
    let db = make_db().await;

    let now = chrono::Utc::now();

    db.create_status_incident("INC1", "t0_abc", None).await?;

    let status = db.get_unresolved_status_incidents().await?;
    assert_eq!(status.len(), 1);

    let status = status.get("INC1").unwrap();

    let status = db.get_status_incident_by_id(&status).await?;

    assert_eq!(
        status,
        Some(StatusIncident {
            incident_id: "INC1".into(),
            updated_at: None,
            resolved_at: None,
            live_fullname: "t0_abc".into()
        })
    );

    db.update_status_incident("INC1", now, Some(now)).await?;

    assert_eq!(db.get_unresolved_status_incidents().await?.len(), 0);

    Ok(())
}

#[tokio::test]
async fn incident_sub_posts_lifecycle() -> DbResult<()> {
    static INC_ID: &str = "INC2";

    let db = make_db().await;

    dbg!();
    db.create_status_incident(INC_ID, "t0_abc", None).await?;
    dbg!();
    db.create_incident_post("sub1", INC_ID, "t3_xyz").await?;
    dbg!();

    let thing = db.get_incident_post("sub1", INC_ID).await?;
    dbg!();
    assert_eq!(
        thing,
        Some(IncidentPostLite {
            post_fullname: "t3_xyz".into(),
            sticky_state: StickyState::NeverStickied
        })
    );

    db.sticky_incident_post("t3_xyz", Some("t3_pri")).await?;
    dbg!();
    let thing = db.get_incident_post_by_id("t3_xyz").await?;
    dbg!();
    assert_eq!(
        thing,
        Some(IncidentPostLite {
            post_fullname: "t3_xyz".into(),
            sticky_state: StickyState::Stickied {
                removed: Some("t3_pri".into())
            }
        })
    );

    let stickied = db.get_stickied_incident_posts("sub1").await?;
    dbg!();
    assert_eq!(stickied.len(), 1);
    assert!(stickied.contains(INC_ID));

    let now = Utc::now();
    db.update_status_incident(INC_ID, now, Some(now)).await?;
    dbg!();

    let to_be_unstickied = db.get_resolved_incidents_still_stickied().await?;
    dbg!();
    assert_eq!(
        to_be_unstickied,
        vec![ResolvedIncidentPost {
            post_fullname: "t3_xyz".into(),
            sticky_state: StickyState::Stickied {
                removed: Some("t3_pri".into())
            },
            resolved_at: now.trunc_subsecs(6)
        }]
    );

    let now = Utc::now().trunc_subsecs(6);
    db.unsticky_incident_post("t3_xyz", now).await?;
    dbg!();
    let to_be_unstickied = db.get_resolved_incidents_still_stickied().await?;
    assert_eq!(to_be_unstickied, Vec::new());

    let thing = db.get_incident_post_by_id("t3_xyz").await?;

    assert_eq!(
        thing,
        Some(IncidentPostLite {
            post_fullname: "t3_xyz".into(),
            sticky_state: StickyState::Unstickied { unstickied_at: now }
        })
    );

    Ok(())
}
