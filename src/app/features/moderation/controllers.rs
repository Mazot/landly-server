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
