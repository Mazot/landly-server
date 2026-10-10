use super::{
    presenters::ImagePresenter,
    repositories::{CreateImageRepositoryInput, ImageRepository},
};
use crate::{error::AppError, utils::storage::StorageService};
use actix_web::HttpResponse;
use serde_json::json;
use std::{env, sync::Arc};
use uuid::Uuid;

pub(super) const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

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

    /// Organisation photos are managed like the organisation itself: by its
    /// creator or a moderator/admin. Without this any user could add images
    /// to (and replace the primary image of) any organisation.
    fn ensure_can_manage_organisation(
        &self,
        organisation_id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<(), AppError> {
        let created_by = self
            .image_repo
            .fetch_organisation_creator(organisation_id)?;
        if created_by == Some(caller_user_id) {
            return Ok(());
        }

        if self
            .image_repo
            .fetch_user_role(caller_user_id)?
            .is_moderator()
        {
            return Ok(());
        }

        Err(AppError::Forbidden(json!({
            "error": "Only the organisation creator or a moderator can manage its images"
        })))
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

        self.ensure_can_manage_organisation(params.organisation_id, params.uploaded_by)?;

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

        // The uploader may withdraw their own image; the organisation's
        // creator and moderators may remove any image of it.
        if image.uploaded_by != caller_user_id {
            self.ensure_can_manage_organisation(image.organisation_id, caller_user_id)?;
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
        let paginated: Vec<_> = images.into_iter().skip(offset).take(limit).collect();

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

        // Choosing the primary image changes how the organisation is shown,
        // so it is up to its creator or a moderator, not the uploader.
        self.ensure_can_manage_organisation(image.organisation_id, caller_user_id)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::images::entities::Image;
    use crate::app::features::images::presenters::ImagePresenterImpl;
    use crate::app::features::user::entities::UserRole;
    use crate::utils::storage::NoOpStorageService;
    use std::sync::Mutex;

    struct StubRepo {
        org_creator: Uuid,
        image_uploader: Uuid,
        caller_role: UserRole,
        primaries: Mutex<Vec<Uuid>>,
    }

    impl StubRepo {
        fn new(caller_role: UserRole) -> Self {
            Self {
                org_creator: Uuid::new_v4(),
                image_uploader: Uuid::new_v4(),
                caller_role,
                primaries: Mutex::new(vec![]),
            }
        }

        fn image(&self) -> Image {
            Image {
                id: Uuid::new_v4(),
                organisation_id: Uuid::new_v4(),
                uploaded_by: self.image_uploader,
                s3_key: "images/org/file.jpg".to_string(),
                s3_bucket: "bucket".to_string(),
                file_name: "file.jpg".to_string(),
                content_type: "image/jpeg".to_string(),
                file_size: 1,
                width: None,
                height: None,
                is_primary: false,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }
        }
    }

    impl ImageRepository for StubRepo {
        fn create_image(&self, _params: CreateImageRepositoryInput) -> Result<Image, AppError> {
            unreachable!("the storage upload fails first in these tests")
        }

        fn delete_image(&self, _id: Uuid) -> Result<Image, AppError> {
            Ok(self.image())
        }

        fn fetch_image(&self, _id: Uuid) -> Result<Image, AppError> {
            Ok(self.image())
        }

        fn fetch_images_by_organisation(&self, _org_id: Uuid) -> Result<Vec<Image>, AppError> {
            Ok(vec![])
        }

        fn set_primary(&self, image_id: Uuid, _org_id: Uuid) -> Result<Image, AppError> {
            self.primaries.lock().unwrap().push(image_id);

            Ok(self.image())
        }

        fn fetch_organisation_creator(&self, _org_id: Uuid) -> Result<Option<Uuid>, AppError> {
            Ok(Some(self.org_creator))
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            Ok(self.caller_role)
        }
    }

    fn usecase(repo: Arc<StubRepo>) -> ImageUsecase {
        ImageUsecase::new(
            repo,
            Arc::new(ImagePresenterImpl::new()),
            Arc::new(NoOpStorageService),
        )
    }

    fn upload_input(uploaded_by: Uuid) -> UploadImageUsecaseInput {
        UploadImageUsecaseInput {
            organisation_id: Uuid::new_v4(),
            uploaded_by,
            data: vec![0xFF, 0xD8, 0xFF],
            file_name: "photo.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
            is_primary: true,
        }
    }

    fn expect_forbidden(result: Result<HttpResponse, AppError>) {
        match result {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.err()),
        }
    }

    /// Regression: any authenticated user could upload a (primary) image to
    /// any organisation.
    #[actix_web::test]
    async fn test_upload_rejects_strangers() {
        let repo = Arc::new(StubRepo::new(UserRole::User));

        expect_forbidden(
            usecase(repo)
                .upload_image(upload_input(Uuid::new_v4()))
                .await,
        );
    }

    /// The creator and moderators pass the ownership check and reach the
    /// storage layer (which the no-op storage then refuses).
    #[actix_web::test]
    async fn test_upload_allows_creator_and_moderator() {
        let repo = Arc::new(StubRepo::new(UserRole::User));
        let creator = repo.org_creator;
        let result = usecase(repo).upload_image(upload_input(creator)).await;
        assert!(
            !matches!(result, Err(AppError::Forbidden(_))),
            "creator must be allowed"
        );

        let repo = Arc::new(StubRepo::new(UserRole::Moderator));
        let result = usecase(repo)
            .upload_image(upload_input(Uuid::new_v4()))
            .await;
        assert!(
            !matches!(result, Err(AppError::Forbidden(_))),
            "moderator must be allowed"
        );
    }

    /// Uploading alone no longer lets a stranger pick the primary image.
    #[test]
    fn test_set_primary_requires_creator_or_moderator() {
        let repo = Arc::new(StubRepo::new(UserRole::User));
        let uploader = repo.image_uploader;
        expect_forbidden(usecase(repo.clone()).set_primary_image(Uuid::new_v4(), uploader));
        assert!(repo.primaries.lock().unwrap().is_empty());

        let creator = repo.org_creator;
        assert!(
            usecase(repo.clone())
                .set_primary_image(Uuid::new_v4(), creator)
                .is_ok()
        );
    }

    #[actix_web::test]
    async fn test_delete_allows_uploader_creator_and_moderator_only() {
        let repo = Arc::new(StubRepo::new(UserRole::User));
        expect_forbidden(
            usecase(repo.clone())
                .delete_image(Uuid::new_v4(), Uuid::new_v4())
                .await,
        );

        for caller in [repo.image_uploader, repo.org_creator] {
            assert!(
                usecase(repo.clone())
                    .delete_image(Uuid::new_v4(), caller)
                    .await
                    .is_ok()
            );
        }

        let repo = Arc::new(StubRepo::new(UserRole::Moderator));
        assert!(
            usecase(repo)
                .delete_image(Uuid::new_v4(), Uuid::new_v4())
                .await
                .is_ok()
        );
    }
}
