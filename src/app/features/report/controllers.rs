use super::requests::CreateReportRequest;
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json},
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

// [authorship] AI-generated (Claude) — the whole report feature is new.
#[utoipa::path(
    post,
    path = "/report/create",
    context_path = "/api",
    request_body = CreateReportRequest,
    responses(
        (status = 200, description = "Report filed", body = super::presenters::ReportContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unknown kind or empty reason", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Report"
)]
pub async fn create_report(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateReportRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.report_usecase.create_report(
        caller,
        form.target_kind.clone(),
        form.target_id,
        form.reason.clone(),
    )
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::test_support::{self as ts, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_create_report() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");

        let (status, report) = call(&app, Method::POST, "/api/report/create", Some(&me.token),
            Some(json!({ "target_kind": "conversation", "target_id": Uuid::new_v4(), "reason": "Spam" }))).await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["status"], "open");
        assert!(
            report.get("reporterId").is_none(),
            "the reporter is not echoed back"
        );

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/report/create",
            Some(&me.token),
            Some(json!({ "target_kind": "planet", "target_id": Uuid::new_v4(), "reason": "?" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/report/create",
            None,
            Some(json!({ "target_kind": "org", "target_id": Uuid::new_v4(), "reason": "x" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
