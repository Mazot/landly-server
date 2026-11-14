//! DB-backed tests of the anti-gaming rules (audit M4): nobody reviews or
//! vouches for their own thing, one check-in per person, `official` is
//! moderator-granted, and a moderator cannot approve their own submission.

use crate::test_support::{self as ts, OrgSpec, TestUser, call, require_db, test_app};
use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceResponse},
    http::{Method, StatusCode},
};
use serde_json::json;
use serial_test::serial;

/// Recommends a person as `recommender`, has `moderator` approve it, then
/// confirms the claim as `claimer` (linking the account → status `claimed`).
/// Returns the person id.
async fn claimed_person<S, B>(
    app: &S,
    recommender: &TestUser,
    moderator: &TestUser,
    claimer: &TestUser,
) -> String
where
    S: Service<actix_http::Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let (status, created) = call(
        app,
        Method::POST,
        "/api/person/create",
        Some(&recommender.token),
        Some(json!({ "name": "Helper", "email": "h@example.test", "consent_given": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = created["person"]["id"].as_str().unwrap().to_string();
    let token = created["claimUrl"]
        .as_str()
        .unwrap()
        .trim_start_matches("/claim/")
        .to_string();

    let (status, body) = call(
        app,
        Method::POST,
        "/api/moderation/approve",
        Some(&moderator.token),
        Some(json!({ "kind": "person", "target_id": id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call(
        app,
        Method::POST,
        &format!("/api/person/claim/{token}/confirm"),
        Some(&claimer.token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "claimed");
    id
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_nobody_reviews_or_vouches_for_their_own_things() {
    let pool = require_db!();
    let app = test_app!(pool);
    let (creator, stranger) = (ts::user(&pool, "user"), ts::user(&pool, "user"));
    let (recommender, claimer, moderator) = (
        ts::user(&pool, "user"),
        ts::user(&pool, "user"),
        ts::user(&pool, "moderator"),
    );

    // Organisation: its submitter cannot review it, anybody else can.
    let org = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(creator.id),
            ..Default::default()
        },
    );
    let review = json!({ "organisation_id": org.id, "rating": 5 });
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/review/create",
        Some(&creator.token),
        Some(review.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/review/create",
        Some(&stranger.token),
        Some(review),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Person: neither the recommender nor the person themselves.
    let id = claimed_person(&app, &recommender, &moderator, &claimer).await;
    for who in [&recommender, &claimer] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/review/create",
            Some(&who.token),
            Some(json!({ "person_id": id, "rating": 5 })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "review: {body}");
        let (status, body) = call(
            &app,
            Method::POST,
            &format!("/api/person/vouch/{id}"),
            Some(&who.token),
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "vouch: {body}");
    }
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/review/create",
        Some(&stranger.token),
        Some(json!({ "person_id": id, "rating": 4 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("/api/person/vouch/{id}"),
        Some(&stranger.token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_one_user_cannot_inflate_checkins() {
    let pool = require_db!();
    let app = test_app!(pool);
    let (spammer, honest) = (ts::user(&pool, "user"), ts::user(&pool, "user"));
    let org = ts::org(&pool, OrgSpec::default());
    let path = format!("/api/organisation/checkin/{}", org.id);

    for n in 0..6 {
        let (status, _) = call(
            &app,
            Method::POST,
            &path,
            Some(&spammer.token),
            Some(json!({ "still_active": true, "tip": format!("tip {n}") })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, body) = call(
        &app,
        Method::POST,
        &path,
        Some(&honest.token),
        Some(json!({ "still_active": false })),
    )
    .await;
    assert_eq!(
        body["community"]["peopleCame"], 2,
        "six taps by one person count once"
    );
    assert_eq!(body["community"]["stillActivePct"], 50.0);
    let tips: Vec<&str> = body["community"]["tips"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tip| tip["text"].as_str().unwrap())
        .collect();
    assert_eq!(tips, ["tip 5"], "one tip per person: the latest");
}

/// A repeat check-in with a blank tip keeps the earlier useful tip.
#[actix_web::test]
#[serial(jwt_env)]
async fn test_blank_repeat_tip_keeps_the_earlier_one() {
    let pool = require_db!();
    let app = test_app!(pool);
    let user = ts::user(&pool, "user");
    let org = ts::org(&pool, OrgSpec::default());
    let path = format!("/api/organisation/checkin/{}", org.id);

    let mut last = json!(null);
    for tip in [json!("bring a passport"), json!(""), json!("   ")] {
        let (status, body) = call(
            &app,
            Method::POST,
            &path,
            Some(&user.token),
            Some(json!({ "tip": tip })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        last = body;
    }

    let tips: Vec<&str> = last["community"]["tips"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tip| tip["text"].as_str().unwrap())
        .collect();
    assert_eq!(tips, ["bring a passport"]);
}

/// The UPDATE itself refuses to touch the badge once a moderator made the
/// row `official`, whatever the usecase read earlier (lost race).
#[actix_web::test]
#[serial(jwt_env)]
async fn test_badge_guard_refuses_to_overwrite_official() {
    use crate::app::features::organisation::entities::{Organisation, UpdateOrganisation};
    use crate::error::AppError;

    let pool = require_db!();
    let org = ts::org(&pool, OrgSpec::default());
    let conn = &mut pool.get().unwrap();
    let change = |added_by: &str| UpdateOrganisation {
        added_by: Some(added_by.to_string()),
        name: None,
        tel: None,
        email: None,
        address: None,
        description: None,
        location_country_id: None,
        organisation_type_id: None,
        updated_at: chrono::Utc::now().naive_utc(),
        latitude: None,
        longitude: None,
        founder_country_id: None,
        city: None,
        website: None,
        telegram: None,
        whatsapp: None,
        services: None,
        languages: None,
        opening_hours: None,
        timezone: None,
        status: None,
        location_source: None,
        location_refreshed_at: None,
        moderation_note: None,
    };

    // Not official yet: a guarded change goes through.
    let updated = Organisation::update(conn, org.id, true, &change("volunteer")).unwrap();
    assert_eq!(updated.added_by.as_deref(), Some("volunteer"));

    // A moderator (unguarded) grants the badge...
    Organisation::update(conn, org.id, false, &change("official")).unwrap();

    // ...and the owner's stale, guarded write must not remove it.
    let result = Organisation::update(conn, org.id, true, &change("community"));
    assert!(matches!(result, Err(AppError::Forbidden(_))), "{result:?}");
    let stored = Organisation::update(conn, org.id, false, &change("official")).unwrap();
    assert_eq!(stored.added_by.as_deref(), Some("official"));
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_official_is_granted_by_moderators_only() {
    let pool = require_db!();
    let app = test_app!(pool);
    let (user, moderator) = (ts::user(&pool, "user"), ts::user(&pool, "moderator"));
    let named =
        |added_by: &str| json!({ "name": format!("Added {}", ts::unique()), "added_by": added_by });

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/organisation/create",
        Some(&user.token),
        Some(named("official")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, org) = call(
        &app,
        Method::POST,
        "/api/organisation/create",
        Some(&user.token),
        Some(named("community")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{org}");
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/organisation/create",
        Some(&moderator.token),
        Some(named("official")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a moderator may vouch for an official source"
    );

    let path = format!("/api/organisation/update/{}", org["id"].as_str().unwrap());
    let (status, _) = call(
        &app,
        Method::PUT,
        &path,
        Some(&user.token),
        Some(json!({ "added_by": "official" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "cannot promote yourself");
    let (status, _) = call(
        &app,
        Method::PUT,
        &path,
        Some(&user.token),
        Some(json!({ "added_by": "community", "description": "same value resent" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "an edit form resends the stored value"
    );
    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(&user.token),
        Some(json!({ "added_by": "robot" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(&moderator.token),
        Some(json!({ "added_by": "official" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["addedBy"], "official");
    let (status, _) = call(
        &app,
        Method::PUT,
        &path,
        Some(&user.token),
        Some(json!({ "added_by": "official", "description": "resent" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "resending the moderator-granted value is not a change"
    );
    let (status, _) = call(
        &app,
        Method::PUT,
        &path,
        Some(&user.token),
        Some(json!({ "added_by": "volunteer" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "cannot drop the mark either");
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_a_moderator_cannot_approve_their_own_submission() {
    let pool = require_db!();
    let app = test_app!(pool);
    let (author, other, admin) = (
        ts::user(&pool, "moderator"),
        ts::user(&pool, "moderator"),
        ts::user(&pool, "admin"),
    );
    let submit = |who: &TestUser| {
        let token = who.token.clone();
        let app = &app;
        async move {
            let (_, org) = call(
                app,
                Method::POST,
                "/api/organisation/create",
                Some(&token),
                Some(json!({ "name": format!("Own {}", ts::unique()) })),
            )
            .await;
            org["id"].as_str().unwrap().to_string()
        }
    };
    let decision = |id: &str| json!({ "kind": "org", "target_id": id });

    let id = submit(&author).await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/moderation/approve",
        Some(&author.token),
        Some(decision(&id)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (_, fetched) = call(
        &app,
        Method::GET,
        &format!("/api/organisation/fetch/{id}"),
        None,
        None,
    )
    .await;
    assert_eq!(fetched["status"], "pending", "nothing was approved");
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/moderation/approve",
        Some(&other.token),
        Some(decision(&id)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a second pair of eyes may approve");

    // Withdrawing your own submission (reject) is harmless and allowed.
    let id = submit(&author).await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/moderation/reject",
        Some(&author.token),
        Some(decision(&id)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Admins are exempt: otherwise a team with one admin could publish nothing.
    let id = submit(&admin).await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/moderation/approve",
        Some(&admin.token),
        Some(decision(&id)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Same for people: a moderator who recommended someone cannot approve them.
    let (_, created) = call(
        &app,
        Method::POST,
        "/api/person/create",
        Some(&author.token),
        Some(json!({ "name": "Friend", "email": "f@example.test", "consent_given": true })),
    )
    .await;
    let person_id = created["person"]["id"].as_str().unwrap();
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/moderation/approve",
        Some(&author.token),
        Some(json!({ "kind": "person", "target_id": person_id })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
