use super::{requests::ImagesListQueryParams, usecases::UploadImageUsecaseInput};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_multipart::Multipart;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Path, Query},
};
use futures::StreamExt;
use serde_json::json;
use std::cmp::min;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/images/upload/{organisation_id}",
    context_path = "/api",
    params(
        ("organisation_id" = Uuid, Path, description = "Organisation ID to attach the image to")
    ),
    responses(
        (status = 200, description = "Image uploaded successfully", body = super::presenters::ImageContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 422, description = "Unprocessable entity (invalid file type / size)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Images"
)]
pub async fn upload_image(
    req: HttpRequest,
    state: Data<AppState>,
    org_id: Path<Uuid>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let caller_user_id = *req
        .extensions()
        .get::<Uuid>()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))?;
    let organisation_id = org_id.into_inner();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut is_primary = false;

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            log::error!("Multipart field error: {:?}", e);
            AppError::UnprocessableEntity(json!({ "error": "Invalid multipart data" }))
        })?;

        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                let fname = field
                    .content_disposition()
                    .and_then(|cd| cd.get_filename())
                    .unwrap_or("unknown")
                    .to_string();

                // Prefer the explicit Content-Type on the part; fall back to
                // extension-based detection so the endpoint works with clients
                // that omit the part's Content-Type header.
                let ct = field
                    .content_type()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| detect_content_type_from_filename(&fname).to_string());

                let mut bytes: Vec<u8> = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| {
                        log::error!("Error reading multipart chunk: {:?}", e);
                        AppError::InternalServerError
                    })?;
                    bytes.extend_from_slice(&data);
                }

                file_name = Some(fname);
                content_type = Some(ct);
                file_bytes = Some(bytes);
            }

            "is_primary" => {
                let mut bytes: Vec<u8> = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|_| AppError::InternalServerError)?;
                    bytes.extend_from_slice(&data);
                }
                is_primary = String::from_utf8_lossy(&bytes).trim() == "true";
            }

            _ => {
                // Drain unknown fields to keep the multipart stream healthy.
                while field.next().await.is_some() {}
            }
        }
    }

    let data = file_bytes.ok_or_else(|| {
        AppError::UnprocessableEntity(json!({ "error": "No file field found in the request" }))
    })?;

    let file_name = file_name.ok_or_else(|| {
        AppError::UnprocessableEntity(json!({ "error": "Could not determine file name" }))
    })?;

    let content_type = content_type.ok_or_else(|| {
        AppError::UnprocessableEntity(json!({ "error": "Could not determine content type" }))
    })?;

    state
        .di_container
        .image_usecase
        .upload_image(UploadImageUsecaseInput {
            organisation_id,
            uploaded_by: caller_user_id,
            data,
            file_name,
            content_type,
            is_primary,
        })
        .await
}

#[utoipa::path(
    delete,
    path = "/images/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Image ID to delete")
    ),
    responses(
        (status = 200, description = "Image deleted successfully"),
        (status = 404, description = "Image not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Images"
)]
pub async fn delete_image(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller_user_id = *req
        .extensions()
        .get::<Uuid>()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))?;

    state
        .di_container
        .image_usecase
        .delete_image(id.into_inner(), caller_user_id)
        .await
}

#[utoipa::path(
    get,
    path = "/images/list/{organisation_id}",
    context_path = "/api",
    params(
        ("organisation_id" = Uuid, Path, description = "Organisation ID"),
        ImagesListQueryParams
    ),
    responses(
        (status = 200, description = "Images list", body = super::presenters::MultipleImagesResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Images"
)]
pub async fn list_images(
    state: Data<AppState>,
    org_id: Path<Uuid>,
    query: Query<ImagesListQueryParams>,
) -> Result<HttpResponse, AppError> {
    let limit = min(query.limit.unwrap_or(20), 100);
    let offset = min(query.offset.unwrap_or(0), 500);

    state
        .di_container
        .image_usecase
        .list_images(org_id.into_inner(), limit, offset)
}

#[utoipa::path(
    get,
    path = "/images/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Image ID to fetch")
    ),
    responses(
        (status = 200, description = "Image fetched successfully", body = super::presenters::ImageContent),
        (status = 404, description = "Image not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Images"
)]
pub async fn fetch_image(state: Data<AppState>, id: Path<Uuid>) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .image_usecase
        .fetch_image(id.into_inner())
}

#[utoipa::path(
    put,
    path = "/images/set-primary/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Image ID to mark as primary for its organisation")
    ),
    responses(
        (status = 200, description = "Primary image updated", body = super::presenters::ImageContent),
        (status = 404, description = "Image not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Images"
)]
pub async fn set_primary_image(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller_user_id = *req
        .extensions()
        .get::<Uuid>()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))?;

    state
        .di_container
        .image_usecase
        .set_primary_image(id.into_inner(), caller_user_id)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detect_content_type_from_filename(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}
