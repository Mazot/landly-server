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
        (status = 403, description = "Caller is neither the organisation creator nor a moderator", body = AppError),
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

    // Reject callers who may not manage this organisation before reading
    // (and buffering) the upload body.
    state
        .di_container
        .image_usecase
        .authorize_upload(organisation_id, caller_user_id)?;

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

                const MAX_UPLOAD_BYTES: usize = super::usecases::MAX_FILE_SIZE;
                let mut bytes: Vec<u8> = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| {
                        log::error!("Error reading multipart chunk: {:?}", e);
                        AppError::InternalServerError
                    })?;
                    if bytes.len() + data.len() > MAX_UPLOAD_BYTES {
                        return Err(AppError::UnprocessableEntity(
                            json!({ "error": "File size exceeds the 10 MB limit." }),
                        ));
                    }
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
        (status = 403, description = "Caller is neither the uploader, the organisation creator nor a moderator", body = AppError),
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
    let limit = query.limit.unwrap_or(20).clamp(0, 100);
    let offset = query.offset.unwrap_or(0).clamp(0, 500);

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
        (status = 403, description = "Caller is neither the organisation creator nor a moderator", body = AppError),
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

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
/// No S3 in tests: storage is the NoOp backend, so uploads that pass every
/// check end in 500 and deletes skip the (best-effort) object removal.
#[cfg(test)]
mod http_tests {
    use crate::app::features::images::entities::CreateImage;
    use crate::data::schema::images;
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use actix_web::test::{self, TestRequest};
    use diesel::prelude::*;
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    fn image(pool: &crate::utils::db::DbPool, org: Uuid, uploader: Uuid, primary: bool) -> Uuid {
        diesel::insert_into(images::table)
            .values(&CreateImage {
                organisation_id: org,
                uploaded_by: uploader,
                s3_key: format!("images/{org}/{}.png", Uuid::new_v4()),
                s3_bucket: "test".to_string(),
                file_name: "photo.png".to_string(),
                content_type: "image/png".to_string(),
                file_size: 68,
                width: None,
                height: None,
                is_primary: primary,
            })
            .returning(images::id)
            .get_result(&mut pool.get().unwrap())
            .unwrap()
    }

    fn multipart(file_name: &str, content_type: &str, data: &[u8]) -> (String, Vec<u8>) {
        let boundary = "landlyTestBoundary";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"is_primary\"\r\n\r\ntrue\r\n--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_upload_authorization_and_validation() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (owner, stranger) = (ts::user(&pool, "user"), ts::user(&pool, "user"));
        let org = ts::org(
            &pool,
            OrgSpec {
                created_by: Some(owner.id),
                ..Default::default()
            },
        );
        let upload = |token: String, ct: &str, file: &str, data: &[u8]| {
            let (content_type, body) = multipart(file, ct, data);
            TestRequest::post()
                .uri(&format!("/api/images/upload/{}", org.id))
                .peer_addr("127.0.0.1:40000".parse().unwrap())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .insert_header(("Content-Type", content_type))
                .set_payload(body)
                .to_request()
        };

        let resp = test::call_service(
            &app,
            upload(stranger.token.clone(), "image/png", "a.png", b"png"),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "rejected before the body is read"
        );
        let resp = test::call_service(
            &app,
            upload(owner.token.clone(), "text/plain", "a.txt", b"hello"),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "unsupported content type"
        );
        let resp = test::call_service(
            &app,
            upload(owner.token.clone(), "image/png", "a.png", b"png"),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "valid upload reaches the NoOp storage"
        );
        let stored: i64 = images::table
            .filter(images::organisation_id.eq(org.id))
            .count()
            .get_result(&mut pool.get().unwrap())
            .unwrap();
        assert_eq!(
            stored, 0,
            "nothing is recorded when the storage write fails"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_list_fetch_primary_and_delete() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (owner, stranger, moderator) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
        );
        let org = ts::org(
            &pool,
            OrgSpec {
                created_by: Some(owner.id),
                ..Default::default()
            },
        );
        let first = image(&pool, org.id, owner.id, true);
        let second = image(&pool, org.id, stranger.id, false);

        let (status, list) = call(
            &app,
            Method::GET,
            &format!("/api/images/list/{}", org.id),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["total"], 2);
        let (status, one) = call(
            &app,
            Method::GET,
            &format!("/api/images/fetch/{first}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(one["isPrimary"], true);
        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/images/fetch/{}", Uuid::new_v4()),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let set_primary = format!("/api/images/set-primary/{second}");
        let (status, _) = call(&app, Method::PUT, &set_primary, Some(&stranger.token), None).await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "uploading an image does not make you an org manager"
        );
        let (status, body) = call(&app, Method::PUT, &set_primary, Some(&owner.token), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (_, old) = call(
            &app,
            Method::GET,
            &format!("/api/images/fetch/{first}"),
            None,
            None,
        )
        .await;
        assert_eq!(
            old["isPrimary"], false,
            "set-primary clears the previous primary"
        );

        // The uploader may delete their own image even without managing the org.
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/images/delete/{second}"),
            Some(&stranger.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let third = image(&pool, org.id, owner.id, false);
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/images/delete/{third}"),
            Some(&stranger.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/images/delete/{third}"),
            Some(&moderator.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, list) = call(
            &app,
            Method::GET,
            &format!("/api/images/list/{}", org.id),
            None,
            None,
        )
        .await;
        assert_eq!(list["total"], 1);
        assert_eq!(list["items"][0]["id"], json!(first));
    }
}
