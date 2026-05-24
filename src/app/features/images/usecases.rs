use super::{
    presenters::ImagePresenter,
    repositories::{CreateImageRepositoryInput, ImageRepository},
};
use crate::{error::AppError, utils::storage::StorageService};
use actix_web::HttpResponse;
use serde_json::json;
use std::{env, sync::Arc};
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/webp",
    "image/gif",
];

#[derive(Clone)]
pub struct ImageUsecase {
    image_repo: Arc<dyn ImageRepository>,
    image_presenter: Arc<dyn ImagePresenter>,
    storage_service: Arc<dyn StorageService>,
}

impl ImageUsecase {
    pub fn new(
        image_repo: Arc<dyn ImageRepository>,
        image_presenter: Arc<dyn ImagePresenter>,
        storage_service: Arc<dyn StorageService>,
    ) -> Self {
        Self {
            image_repo,
            image_presenter,
            storage_service,
        }
    }

    /// Upload a file to object storage and persist its metadata in the database.
    ///
    /// Validation performed:
    /// - Content-type must be one of `ALLOWED_CONTENT_TYPES`.
    /// - File must not exceed `MAX_FILE_SIZE`.
    pub async fn upload_image(
        &self,
        params: UploadImageUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        // --- Validation --------------------------------------------------

        let normalised_ct = params.content_type.to_lowercase();

        if !ALLOWED_CONTENT_TYPES.contains(&normalised_ct.as_str()) {
            return Err(AppError::UnprocessableEntity(serde_json::json!({
                "error": format!(
                    "Unsupported content type '{}'. Allowed types: jpeg, jpg, png, webp, gif.",
                    params.content_type
                )
            })));
        }

        if params.data.len() > MAX_FILE_SIZE {
            return Err(AppError::UnprocessableEntity(serde_json::json!({
                "error": "File size exceeds the 10 MB limit."
            })));
        }

        // --- Build storage key -------------------------------------------

        let ext = content_type_to_ext(&normalised_ct);
        let object_key = format!(
            "images/{}/{}.{}",
            params.organisation_id,
            Uuid::new_v4(),
            ext
        );

        let file_size = params.data.len() as i64;

        // --- Upload to storage -------------------------------------------

        self.storage_service
            .upload(&object_key, params.data, &normalised_ct)
            .await?;

        // --- Persist metadata in DB --------------------------------------

        let bucket = env::var("S3_BUCKET").map_err(|_| {
            log::error!("S3_BUCKET environment variable is not set");
            AppError::InternalServerError
        })?;

        let image = self.image_repo.create_image(CreateImageRepositoryInput {
            organisation_id: params.organisation_id,
            uploaded_by: params.uploaded_by,
            s3_key: object_key.clone(),
            s3_bucket: bucket,
            file_name: params.file_name,
            content_type: normalised_ct,
            file_size,
            width: None,
            height: None,
            is_primary: params.is_primary,
        })?;

        // --- Build response ----------------------------------------------

        let url = self.storage_service.get_public_url(&image.s3_key);

        Ok(self.image_presenter.to_single_json(image, url))
    }

    /// Remove an image from the database and (best-effort) from object storage.
    ///
    /// The DB record is deleted first so that no stale reference remains even
    /// if the storage delete fails.  Storage failures are logged but do **not**
    /// cause the endpoint to return an error.
    pub async fn delete_image(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        // Fetch before deleting so we have the s3_key and can verify ownership.
        let image = self.image_repo.fetch_image(id)?;

        if image.uploaded_by != caller_user_id {
            return Err(AppError::Forbidden(json!({
                "error": "You do not have permission to delete this image"
            })));
        }

        let s3_key = image.s3_key.clone();

        // Remove from DB first.
        self.image_repo.delete_image(id)?;

        // Best-effort storage deletion.
        if let Err(e) = self.storage_service.delete(&s3_key).await {
            log::error!(
                "Failed to delete object '{}' from storage (DB record already removed): {:?}",
                s3_key,
                e
            );
        }

        Ok(self.image_presenter.to_http_res())
    }

    /// Return all images belonging to an organisation.
    pub fn list_images(
        &self,
        organisation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<HttpResponse, AppError> {
        let images = self
            .image_repo
            .fetch_images_by_organisation(organisation_id)?;

        let total = images.len() as i64;
        let offset = offset.max(0) as usize;
        let limit = limit.max(0) as usize;

        // Apply in-memory pagination (the repository loads the full list so we
        // can still benefit from the cache; for large datasets a DB-level LIMIT
        // / OFFSET query would be more appropriate).
        let paginated: Vec<_> = images
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        let urls: Vec<String> = paginated
            .iter()
            .map(|img| self.storage_service.get_public_url(&img.s3_key))
            .collect();

        Ok(self.image_presenter.to_multi_json(paginated, urls, total))
    }

    /// Fetch a single image by its ID.
    pub fn fetch_image(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let image = self.image_repo.fetch_image(id)?;
        let url = self.storage_service.get_public_url(&image.s3_key);

        Ok(self.image_presenter.to_single_json(image, url))
    }

    /// Mark an image as the primary image for its organisation.
    ///
    /// Fetches the image first to resolve the `organisation_id`, then delegates
    /// to the repository which runs the swap in a single transaction.
    pub fn set_primary_image(
        &self,
        image_id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        // Resolve the organisation the image belongs to and verify ownership.
        let image = self.image_repo.fetch_image(image_id)?;

        if image.uploaded_by != caller_user_id {
            return Err(AppError::Forbidden(json!({
                "error": "You do not have permission to set this image as primary"
            })));
        }

        let org_id = image.organisation_id;

        let updated = self.image_repo.set_primary(image_id, org_id)?;
        let url = self.storage_service.get_public_url(&updated.s3_key);

        Ok(self.image_presenter.to_single_json(updated, url))
    }
}

// ---------------------------------------------------------------------------
// Usecase input types
// ---------------------------------------------------------------------------

pub struct UploadImageUsecaseInput {
    pub organisation_id: Uuid,
    /// ID of the authenticated user performing the upload.
    pub uploaded_by: Uuid,
    /// Raw file bytes.
    pub data: Vec<u8>,
    pub file_name: String,
    /// MIME content-type reported by the multipart field (e.g. `image/jpeg`).
    pub content_type: String,
    /// Whether this image should immediately become the primary image for the
    /// organisation.
    pub is_primary: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn content_type_to_ext(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}
