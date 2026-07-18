use super::{requests::CreateCorridorRequest, usecases::CreateCorridorUsecaseInput};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path},
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

#[utoipa::path(
    post,
    path = "/corridor/create",
    context_path = "/api",
    request_body = CreateCorridorRequest,
    responses(
        (status = 200, description = "Corridor created successfully", body = super::presenters::CorridorContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn create_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateCorridorRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    if form.from_country_id == form.to_country_id {
        return Err(AppError::UnprocessableEntity(
            json!({ "error": "Corridor countries must be different" }),
        ));
    }

    state
        .di_container
        .corridor_usecase
        .create_corridor(CreateCorridorUsecaseInput {
            user_id,
            from_country_id: form.from_country_id,
            to_country_id: form.to_country_id,
            is_default: form.is_default.unwrap_or(false),
        })
}

#[utoipa::path(
    get,
    path = "/corridor/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Corridors of the authenticated user", body = super::presenters::MultipleCorridorsResponse),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn list_corridors(
    req: HttpRequest,
    state: Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.corridor_usecase.list_corridors(user_id)
}

#[utoipa::path(
    put,
    path = "/corridor/set-default/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to make default")
    ),
    responses(
        (status = 200, description = "Corridor set as default", body = super::presenters::CorridorContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn set_default_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .set_default_corridor(id.into_inner(), user_id)
}

#[utoipa::path(
    delete,
    path = "/corridor/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to delete")
    ),
    responses(
        (status = 200, description = "Corridor deleted successfully"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn delete_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .delete_corridor(id.into_inner(), user_id)
}

#[utoipa::path(
    get,
    path = "/corridor/stats/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to fetch stats for")
    ),
    responses(
        (status = 200, description = "Live-organisation counters for the corridor destination", body = super::presenters::CorridorStatsContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn fetch_corridor_stats(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .fetch_corridor_stats(id.into_inner(), user_id)
}
