//! DB-backed tests of the input size limits (audit M3): every free-text
//! field, list and JSON document is bounded, at exactly the documented limit
//! the request still passes, one past it is a 422 naming the field, and a
//! JSON body over 64 KB is refused before any handler runs.

use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
use crate::utils::validation::{MAX_JSON_BODY_BYTES, limits};
use actix_web::http::{Method, StatusCode};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

fn text(len: usize) -> String {
    "x".repeat(len)
}

/// 422 whose message names `field`.
fn assert_limit(status: StatusCode, body: &Value, field: &str) {
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}: {body}");
    let message = body["error"].as_str().unwrap_or_default();
    assert!(
        message.starts_with(field),
        "{field}: unexpected message {body}"
    );
}

fn org(extra: Value) -> Value {
    let mut body = json!({ "name": format!("Limits {}", ts::unique()) });
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    body
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_organisation_fields_are_bounded() {
    let pool = require_db!();
    let app = test_app!(pool);
    let me = ts::user(&pool, "user");
    let create = |body: Value| {
        let (app, token) = (&app, me.token.clone());
        async move {
            call(
                app,
                Method::POST,
                "/api/organisation/create",
                Some(&token),
                Some(body),
            )
            .await
        }
    };

    let too_long = [
        ("name", limits::ORG_NAME),
        ("tel", limits::PHONE),
        ("email", limits::EMAIL),
        ("address", limits::ADDRESS),
        ("description", limits::DESCRIPTION),
        ("city", limits::CITY),
        ("website", limits::URL),
        ("telegram", limits::HANDLE),
        ("whatsapp", limits::HANDLE),
        ("timezone", limits::TIMEZONE),
    ];
    for (field, max) in too_long {
        let (status, body) = create(org(json!({ field: text(max + 1) }))).await;
        assert_limit(status, &body, field);
    }

    // Exactly at the limit passes — counted in characters, so Cyrillic text of
    // the maximum length (twice the bytes) is fine.
    let (status, body) = create(
        json!({ "name": "я".repeat(limits::ORG_NAME), "description": text(limits::DESCRIPTION) }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = create(org(
        json!({ "services": vec!["s"; limits::TAGS_PER_LIST + 1] }),
    ))
    .await;
    assert_limit(status, &body, "services");
    let (status, body) = create(org(json!({ "languages": [text(limits::TAG + 1)] }))).await;
    assert_limit(status, &body, "languages");
    let (status, body) = create(org(json!({ "opening_hours": [["09:00", "17:00"]] }))).await;
    assert_limit(status, &body, "opening_hours");
    let (status, body) = create(org(
        json!({ "opening_hours": { "mon": text(limits::JSON_DOCUMENT_BYTES) } }),
    ))
    .await;
    assert_limit(status, &body, "opening_hours");

    let (status, body) = create(org(json!({
        "services": vec!["Help"; limits::TAGS_PER_LIST],
        "opening_hours": { "mon": [["09:00", "17:00"]] },
    })))
    .await;
    assert_eq!(status, StatusCode::OK, "limits are inclusive: {body}");
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_organisation_update_and_filters_are_bounded() {
    let pool = require_db!();
    let app = test_app!(pool);
    let me = ts::user(&pool, "user");
    let org_row = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(me.id),
            ..Default::default()
        },
    );
    let path = format!("/api/organisation/update/{}", org_row.id);

    for (field, max) in [
        ("description", limits::DESCRIPTION),
        ("name", limits::ORG_NAME),
    ] {
        let (status, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&me.token),
            Some(json!({ field: text(max + 1) })),
        )
        .await;
        assert_limit(status, &body, field);
    }
    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(&me.token),
        Some(json!({ "services": vec!["s"; limits::TAGS_PER_LIST + 1] })),
    )
    .await;
    assert_limit(status, &body, "services");

    let (status, body) = call(
        &app,
        Method::GET,
        &format!("/api/organisation/list?name={}", text(limits::FILTER + 1)),
        None,
        None,
    )
    .await;
    assert_limit(status, &body, "name");
    let types = vec!["t"; limits::FILTER_LIST_ITEMS + 1].join(",");
    let (status, body) = call(
        &app,
        Method::GET,
        &format!("/api/organisation/search?lat=1&lng=1&radius_km=5&types={types}"),
        None,
        None,
    )
    .await;
    assert_limit(status, &body, "types");

    // Type slugs may be as long as the slug limit; one more is refused.
    let slug = |len: usize| "s".repeat(len);
    let search = |len: usize| {
        format!(
            "/api/organisation/search?lat=1&lng=1&radius_km=5&types={}",
            slug(len)
        )
    };
    let (status, body) = call(&app, Method::GET, &search(limits::SLUG), None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call(&app, Method::GET, &search(limits::SLUG + 1), None, None).await;
    assert_limit(status, &body, "types");

    let (status, body) = call(
        &app,
        Method::POST,
        &format!("/api/organisation/checkin/{}", org_row.id),
        Some(&me.token),
        Some(json!({ "tip": text(limits::CHECKIN_TIP + 1) })),
    )
    .await;
    assert_limit(status, &body, "tip");
    let (status, body) = call(
        &app,
        Method::POST,
        &format!("/api/organisation/checkin/{}", org_row.id),
        Some(&me.token),
        Some(json!({ "tip": text(limits::CHECKIN_TIP) })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_people_reviews_reports_and_bookmarks_are_bounded() {
    let pool = require_db!();
    let app = test_app!(pool);
    let me = ts::user(&pool, "user");
    let org_row = ts::org(&pool, OrgSpec::default());
    let person = |extra: Value| {
        let mut body =
            json!({ "name": "Helper", "email": "h@example.test", "consent_given": true });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        body
    };

    for (field, max) in [
        ("name", limits::PERSON_NAME),
        ("bio", limits::BIO),
        ("city", limits::CITY),
        ("email", limits::EMAIL),
        ("whatsapp", limits::HANDLE),
    ] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/person/create",
            Some(&me.token),
            Some(person(json!({ field: text(max + 1) }))),
        )
        .await;
        assert_limit(status, &body, field);
    }
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/person/create",
        Some(&me.token),
        Some(person(json!({ "skills": vec!["s"; limits::SKILLS + 1] }))),
    )
    .await;
    assert_limit(status, &body, "skills");
    let ids: Vec<Uuid> = (0..=limits::LANGUAGE_IDS).map(|_| Uuid::new_v4()).collect();
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/person/create",
        Some(&me.token),
        Some(person(json!({ "language_ids": ids }))),
    )
    .await;
    assert_limit(status, &body, "language_ids");
    let (status, body) = call(
        &app,
        Method::GET,
        &format!("/api/person/list?city={}", text(limits::FILTER + 1)),
        None,
        None,
    )
    .await;
    assert_limit(status, &body, "city");

    let review = |extra: Value| {
        let mut body = json!({ "organisation_id": org_row.id, "rating": 4 });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        body
    };
    for (field, max) in [
        ("text", limits::REVIEW_TEXT),
        ("topic", limits::REVIEW_TOPIC),
    ] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/review/create",
            Some(&me.token),
            Some(review(json!({ field: text(max + 1) }))),
        )
        .await;
        assert_limit(status, &body, field);
    }
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/review/create",
        Some(&me.token),
        Some(review(json!({ "text": text(limits::REVIEW_TEXT) }))),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call(&app, Method::POST, "/api/report/create", Some(&me.token), Some(json!({ "target_kind": "org", "target_id": org_row.id, "reason": text(limits::REPORT_REASON + 1) }))).await;
    assert_limit(status, &body, "reason");

    for (field, max) in [
        ("note", limits::SAVED_NOTE),
        ("list_name", limits::SAVED_LIST_NAME),
    ] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/saved/create",
            Some(&me.token),
            Some(json!({ "kind": "org", "target_id": Uuid::new_v4(), field: text(max + 1) })),
        )
        .await;
        assert_limit(status, &body, field);
    }
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_user_documents_and_admin_inputs_are_bounded() {
    let pool = require_db!();
    let app = test_app!(pool);
    let (me, admin) = (ts::user(&pool, "user"), ts::user(&pool, "admin"));

    for settings in [
        json!([1, 2, 3]),
        json!("text"),
        json!({ "digest": text(limits::JSON_DOCUMENT_BYTES) }),
    ] {
        let (status, body) = call(
            &app,
            Method::PUT,
            "/api/user/me/notifications",
            Some(&me.token),
            Some(json!({ "notification_settings": settings })),
        )
        .await;
        assert_limit(status, &body, "notification_settings");
    }
    let ids: Vec<Uuid> = (0..=limits::LANGUAGE_IDS).map(|_| Uuid::new_v4()).collect();
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/user/languages",
        Some(&me.token),
        Some(json!({ "languages_ids": ids })),
    )
    .await;
    assert_limit(status, &body, "languages_ids");

    for (field, max) in [
        ("org_type", limits::ORG_TYPE_KEY),
        ("color", limits::COLOR),
        ("title", limits::ORG_TYPE_TITLE),
        ("slug", limits::SLUG),
    ] {
        let mut body = json!({ "org_type": "t", "color": "#000", "title": "T" });
        body[field] = json!(text(max + 1));
        let (status, resp) = call(
            &app,
            Method::POST,
            "/api/common/org_types",
            Some(&admin.token),
            Some(body),
        )
        .await;
        assert_limit(status, &resp, field);
    }
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/country-connection/create",
        Some(&admin.token),
        Some(json!({ "common_info": text(limits::COMMON_INFO + 1) })),
    )
    .await;
    assert_limit(status, &body, "common_info");
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_oversized_json_bodies_are_refused() {
    let pool = require_db!();
    let app = test_app!(pool);
    let me = ts::user(&pool, "user");

    let huge = json!({ "name": "Huge", "description": text(MAX_JSON_BODY_BYTES) });
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/organisation/create",
        Some(&me.token),
        Some(huge),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "refused by the extractor, before any validation"
    );

    // The cap does not get in the way of a maximal real form.
    let big = org(
        json!({ "description": text(limits::DESCRIPTION), "services": vec![text(limits::TAG); limits::TAGS_PER_LIST] }),
    );
    assert!(big.to_string().len() < MAX_JSON_BODY_BYTES / 2);
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/organisation/create",
        Some(&me.token),
        Some(big),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
