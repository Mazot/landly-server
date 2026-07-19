use super::{
    requests::{CreateReviewRequest, ListReviewsQueryRequest},
    usecases::CreateReviewUsecaseInput,
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
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

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    post,
    path = "/review/create",
    context_path = "/api",
    request_body = CreateReviewRequest,
    responses(
        (status = 200, description = "Review created; target aggregates refreshed", body = super::presenters::ReviewContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Reviews disabled by the person", body = AppError),
        (status = 422, description = "Unprocessable entity (bad target/rating/duplicate)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Review"
)]
pub async fn create_review(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateReviewRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .review_usecase
        .create_review(CreateReviewUsecaseInput {
            author_id: caller,
            organisation_id: form.organisation_id,
            person_id: form.person_id,
            rating: form.rating,
            topic: form.topic.clone(),
            text: form.text.clone(),
        })
}

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    get,
    path = "/review/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Reviews of one target", body = super::presenters::MultipleReviewsResponse),
        (status = 422, description = "Exactly one target must be given", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListReviewsQueryRequest),
    tag = "Review"
)]
pub async fn list_reviews(
    state: Data<AppState>,
    query: Query<ListReviewsQueryRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.review_usecase.list_reviews(
        query.organisation_id,
        query.person_id,
        query.limit.unwrap_or(20).clamp(0, 100),
        query.offset.unwrap_or(0).clamp(0, 500),
    )
}

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    delete,
    path = "/review/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Review ID to delete")
    ),
    responses(
        (status = 200, description = "Review deleted; target aggregates refreshed"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (author or moderator only)", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Review"
)]
pub async fn delete_review(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .review_usecase
        .delete_review(id.into_inner(), caller)
}
