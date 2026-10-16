use super::requests::{ModerationDecisionRequest, ModerationQueueQueryRequest};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Query},
};
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

// [authorship] AI-generated (Claude) — the whole moderation feature is new.
#[utoipa::path(
    get,
    path = "/moderation/queue",
    context_path = "/api",
    responses(
        (status = 200, description = "Pending orgs + people with last events and open reports", body = super::presenters::ModerationQueueResponse),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (moderator only)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ModerationQueueQueryRequest),
    tag = "Moderation"
)]
pub async fn fetch_queue(
    req: HttpRequest,
    state: Data<AppState>,
    query: Query<ModerationQueueQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .moderation_usecase
        .fetch_queue(caller, query.kind.clone())
}

// [authorship] AI-generated (Claude) — the whole moderation feature is new.
#[utoipa::path(
    post,
    path = "/moderation/approve",
    context_path = "/api",
    request_body = ModerationDecisionRequest,
    responses(
        (status = 200, description = "Approved: org goes live, person moves to awaiting"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (moderator only)", body = AppError),
        (status = 404, description = "Target not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Moderation"
)]
pub async fn approve(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<ModerationDecisionRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.moderation_usecase.approve(
        caller,
        form.kind.clone(),
        form.target_id,
        form.note.clone(),
    )
}

// [authorship] AI-generated (Claude) — the whole moderation feature is new.
#[utoipa::path(
    post,
    path = "/moderation/request-changes",
    context_path = "/api",
    request_body = ModerationDecisionRequest,
    responses(
        (status = 200, description = "Changes requested; item stays pending with the note"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (moderator only)", body = AppError),
        (status = 404, description = "Target not found", body = AppError),
        (status = 422, description = "Note is required", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Moderation"
)]
pub async fn request_changes(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<ModerationDecisionRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.moderation_usecase.request_changes(
        caller,
        form.kind.clone(),
        form.target_id,
        form.note.clone(),
    )
}

// [authorship] AI-generated (Claude) — the whole moderation feature is new.
#[utoipa::path(
    post,
    path = "/moderation/reject",
    context_path = "/api",
    request_body = ModerationDecisionRequest,
    responses(
        (status = 200, description = "Rejected: org marked rejected, person declined"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (moderator only)", body = AppError),
        (status = 404, description = "Target not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Moderation"
)]
pub async fn reject(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<ModerationDecisionRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.moderation_usecase.reject(
        caller,
        form.kind.clone(),
        form.target_id,
        form.note.clone(),
    )
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::data::schema::{organisations, reports};
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use diesel::prelude::*;
    use serde_json::json;
    use serial_test::serial;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_queue_and_decisions_on_organisations() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (author, moderator, plain) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
            ts::user(&pool, "user"),
        );
        let (status, org) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&author.token),
            Some(json!({ "name": format!("Queued {}", ts::unique()) })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let id = org["id"].as_str().unwrap().to_string();
        call(
            &app,
            Method::POST,
            "/api/report/create",
            Some(&plain.token),
            Some(json!({ "target_kind": "org", "target_id": id, "reason": "Wrong address" })),
        )
        .await;

        let (status, _) = call(
            &app,
            Method::GET,
            "/api/moderation/queue",
            Some(&plain.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, queue) = call(
            &app,
            Method::GET,
            "/api/moderation/queue?kind=org",
            Some(&moderator.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let item = queue["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["targetId"] == id)
            .expect("pending org is queued");
        assert_eq!(item["openReports"], 1);
        assert_eq!(item["lastEvent"]["action"], "submitted");
        let (status, _) = call(
            &app,
            Method::GET,
            "/api/moderation/queue?kind=planet",
            Some(&moderator.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let decision = json!({ "kind": "org", "target_id": id });
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/request-changes",
            Some(&moderator.token),
            Some(decision.clone()),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a note is required"
        );
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/request-changes",
            Some(&moderator.token),
            Some(json!({ "kind": "org", "target_id": id, "note": "Add opening hours" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let conn = &mut pool.get().unwrap();
        let (status_after, note): (String, Option<String>) = organisations::table
            .find(id.parse::<uuid::Uuid>().unwrap())
            .select((organisations::status, organisations::moderation_note))
            .first(conn)
            .unwrap();
        assert_eq!(
            (status_after.as_str(), note.as_deref()),
            ("pending", Some("Add opening hours"))
        );

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/approve",
            Some(&plain.token),
            Some(decision.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/approve",
            Some(&moderator.token),
            Some(decision),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, queue) = call(
            &app,
            Method::GET,
            "/api/moderation/queue",
            Some(&moderator.token),
            None,
        )
        .await;
        assert!(
            !queue["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["targetId"] == id),
            "approved items leave the queue"
        );

        let open: i64 = reports::table
            .filter(reports::status.eq("open"))
            .count()
            .get_result(conn)
            .unwrap();
        assert!(open >= 1);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_reject_organisation_and_unknown_target() {
        let pool = require_db!();
        let app = test_app!(pool);
        let moderator = ts::user(&pool, "admin");
        let org = ts::org(
            &pool,
            OrgSpec {
                status: Some("pending"),
                ..Default::default()
            },
        );

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/reject",
            Some(&moderator.token),
            Some(json!({ "kind": "org", "target_id": org.id, "note": "Spam" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let status_after: String = organisations::table
            .find(org.id)
            .select(organisations::status)
            .first(&mut pool.get().unwrap())
            .unwrap();
        assert_eq!(status_after, "rejected");

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/approve",
            Some(&moderator.token),
            Some(json!({ "kind": "org", "target_id": uuid::Uuid::new_v4() })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/approve",
            Some(&moderator.token),
            Some(json!({ "kind": "galaxy", "target_id": org.id })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}
