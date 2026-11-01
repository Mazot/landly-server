//! DB-backed tests of storage cleanup (audit M7): deleting an organisation
//! removes its image objects from the bucket — and only its own.

use crate::app::drivers::middlewares::rate_limit::RateLimits;
use crate::data::schema::images;
use crate::test_support::{self as ts, OrgSpec, RecordingStorage, call, require_db, test_app};
use actix_web::http::{Method, StatusCode};
use diesel::prelude::*;
use serial_test::serial;
use uuid::Uuid;

fn image_rows(pool: &crate::utils::db::DbPool, org_id: Uuid) -> i64 {
    images::table
        .filter(images::organisation_id.eq(org_id))
        .count()
        .get_result(&mut pool.get().unwrap())
        .unwrap()
}

#[actix_web::test]
#[serial(jwt_env)]
async fn test_deleting_an_organisation_removes_its_objects_only() {
    let pool = require_db!();
    let storage = RecordingStorage::new();
    let app = test_app!(pool, None, RateLimits::relaxed(), storage.clone());
    let owner = ts::user(&pool, "user");
    let org = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(owner.id),
            ..Default::default()
        },
    );
    let other = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(owner.id),
            ..Default::default()
        },
    );
    let keys = [
        format!("images/{}/{}.png", org.id, Uuid::new_v4()),
        format!("images/{}/{}.png", org.id, Uuid::new_v4()),
    ];
    let kept = format!("images/{}/{}.png", other.id, Uuid::new_v4());
    for key in &keys {
        ts::image(&pool, org.id, owner.id, key);
    }
    ts::image(&pool, other.id, owner.id, &kept);

    let (status, body) = call(
        &app,
        Method::DELETE,
        &format!("/api/organisation/delete/{}", org.id),
        Some(&owner.token),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let mut deleted = storage.deleted();
    deleted.sort();
    let mut expected = keys.to_vec();
    expected.sort();
    assert_eq!(
        deleted, expected,
        "exactly the deleted organisation's objects"
    );
    assert_eq!(image_rows(&pool, org.id), 0);
    assert_eq!(
        image_rows(&pool, other.id),
        1,
        "other organisations are untouched"
    );
}

/// The bucket being down must not stop the delete — the sweep script picks the
/// leftovers up later.
#[actix_web::test]
#[serial(jwt_env)]
async fn test_organisation_delete_succeeds_when_storage_is_down() {
    let pool = require_db!();
    let storage = RecordingStorage::failing_deletes();
    let app = test_app!(pool, None, RateLimits::relaxed(), storage.clone());
    let owner = ts::user(&pool, "user");
    let org = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(owner.id),
            ..Default::default()
        },
    );
    let key = format!("images/{}/{}.png", org.id, Uuid::new_v4());
    ts::image(&pool, org.id, owner.id, &key);

    let (status, body) = call(
        &app,
        Method::DELETE,
        &format!("/api/organisation/delete/{}", org.id),
        Some(&owner.token),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        storage.delete_attempts(),
        vec![key],
        "cleanup was attempted"
    );
    assert!(storage.deleted().is_empty());
    assert_eq!(image_rows(&pool, org.id), 0);
}

/// A refused delete must not touch the bucket.
#[actix_web::test]
#[serial(jwt_env)]
async fn test_refused_organisation_delete_leaves_objects_alone() {
    let pool = require_db!();
    let storage = RecordingStorage::new();
    let app = test_app!(pool, None, RateLimits::relaxed(), storage.clone());
    let (owner, stranger) = (ts::user(&pool, "user"), ts::user(&pool, "user"));
    let org = ts::org(
        &pool,
        OrgSpec {
            created_by: Some(owner.id),
            ..Default::default()
        },
    );
    ts::image(
        &pool,
        org.id,
        owner.id,
        &format!("images/{}/{}.png", org.id, Uuid::new_v4()),
    );

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/organisation/delete/{}", org.id),
        Some(&stranger.token),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(storage.delete_attempts().is_empty());
    assert_eq!(image_rows(&pool, org.id), 1);
}
