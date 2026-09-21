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
