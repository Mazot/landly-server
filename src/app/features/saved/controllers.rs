use super::{
    requests::{CreateSavedRequest, ListSavedQueryRequest},
    usecases::CreateSavedUsecaseInput,
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

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    post,
    path = "/saved/create",
    context_path = "/api",
    request_body = CreateSavedRequest,
    responses(
        (status = 200, description = "Bookmark saved", body = super::presenters::SavedItemContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unknown kind or already saved", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn create_saved(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateSavedRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .create_saved(CreateSavedUsecaseInput {
            user_id: caller,
            kind: form.kind.clone(),
            target_id: form.target_id,
            note: form.note.clone(),
            list_name: form.list_name.clone(),
        })
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    delete,
    path = "/saved/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Saved item ID")
    ),
    responses(
        (status = 200, description = "Bookmark removed"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 404, description = "Not found (or not yours)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn delete_saved(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .delete_saved(id.into_inner(), caller)
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    get,
    path = "/saved/list",
    context_path = "/api",
    responses(
        (status = 200, description = "The caller's bookmarks", body = super::presenters::MultipleSavedItemsResponse),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListSavedQueryRequest),
    tag = "Saved"
)]
pub async fn list_saved(
    req: HttpRequest,
    state: Data<AppState>,
    query: Query<ListSavedQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .list_saved(caller, query.kind.clone())
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    get,
    path = "/saved/counts",
    context_path = "/api",
    responses(
        (status = 200, description = "Bookmark counters per kind", body = super::presenters::SavedCountsContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn counts_saved(
    req: HttpRequest,
    state: Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.saved_usecase.counts_saved(caller)
}
