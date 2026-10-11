use super::entities::{CreateImage, Image};
use crate::app::features::user::entities::{User, UserRole};
use crate::{
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub trait ImageRepository: Send + Sync + 'static {
    fn create_image(&self, params: CreateImageRepositoryInput) -> Result<Image, AppError>;

    /// Delete the DB record and return it so callers can obtain the `s3_key`
    /// for subsequent storage cleanup.
    fn delete_image(&self, id: Uuid) -> Result<Image, AppError>;

    fn fetch_image(&self, id: Uuid) -> Result<Image, AppError>;

    fn fetch_images_by_organisation(&self, org_id: Uuid) -> Result<Vec<Image>, AppError>;

    fn set_primary(&self, image_id: Uuid, org_id: Uuid) -> Result<Image, AppError>;

    /// `created_by` of the organisation (404 when it does not exist).
    fn fetch_organisation_creator(&self, org_id: Uuid) -> Result<Option<Uuid>, AppError>;

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError>;
}

// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ImageRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl ImageRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }
}

impl ImageRepository for ImageRepositoryImpl {
    fn create_image(&self, params: CreateImageRepositoryInput) -> Result<Image, AppError> {
        let conn = &mut self.pool.get()?;

        let image = Image::create(
            conn,
            &CreateImage {
                organisation_id: params.organisation_id,
                uploaded_by: params.uploaded_by,
                s3_key: params.s3_key,
                s3_bucket: params.s3_bucket,
                file_name: params.file_name,
                content_type: params.content_type,
                file_size: params.file_size,
                width: params.width,
                height: params.height,
                is_primary: params.is_primary,
            },
        )?;

        // Invalidate organisation image list and prime the per-image cache.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::images_pattern());
        let _ = self
            .cache_service
            .set::<Image>(&CacheKeys::image_by_id(&image.id), &image, None);

        Ok(image)
    }

    fn delete_image(&self, id: Uuid) -> Result<Image, AppError> {
        let conn = &mut self.pool.get()?;

        let image = Image::delete(conn, id)?;

        let _ = self.cache_service.delete(&CacheKeys::image_by_id(&id));
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::images_pattern());

        Ok(image)
    }

    fn fetch_image(&self, id: Uuid) -> Result<Image, AppError> {
        let cache_key = CacheKeys::image_by_id(&id);

        if let Some(cached) = self.cache_service.get::<Image>(&cache_key)? {
            return Ok(cached);
        }

        let conn = &mut self.pool.get()?;
        let image = Image::fetch_by_id(conn, id)?;

        let _ =
            self.cache_service
                .set::<Image>(&cache_key, &image, Some(Duration::from_secs(3600)));

        Ok(image)
    }

    fn fetch_images_by_organisation(&self, org_id: Uuid) -> Result<Vec<Image>, AppError> {
        let cache_key = CacheKeys::images_by_organisation(&org_id);

        if let Some(cached) = self.cache_service.get::<Vec<Image>>(&cache_key)? {
            return Ok(cached);
        }

        let conn = &mut self.pool.get()?;
        let images = Image::fetch_by_organisation(conn, org_id)?;

        let _ = self.cache_service.set::<Vec<Image>>(
            &cache_key,
            &images,
            Some(Duration::from_secs(5 * 60)),
        );

        Ok(images)
    }

    fn set_primary(&self, image_id: Uuid, org_id: Uuid) -> Result<Image, AppError> {
        let conn = &mut self.pool.get()?;

        let image = Image::set_primary(conn, image_id, org_id)?;

        // Invalidate everything for this organisation so lists are consistent.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::images_pattern());
        let _ = self
            .cache_service
            .set::<Image>(&CacheKeys::image_by_id(&image.id), &image, None);

        Ok(image)
    }

    fn fetch_organisation_creator(&self, org_id: Uuid) -> Result<Option<Uuid>, AppError> {
        use crate::data::schema::organisations;
        use diesel::prelude::*;

        let conn = &mut self.pool.get()?;
        let created_by = organisations::table
            .find(org_id)
            .select(organisations::created_by)
            .first::<Option<Uuid>>(conn)?;

        Ok(created_by)
    }

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError> {
        let conn = &mut self.pool.get()?;

        User::fetch_role(conn, user_id)
    }
}

// ---------------------------------------------------------------------------
// Repository input types
// ---------------------------------------------------------------------------

pub struct CreateImageRepositoryInput {
    /// Organisation the photo belongs to.
    pub organisation_id: Uuid,
    /// User who uploaded it (the authenticated caller).
    pub uploaded_by: Uuid,
    /// Object key in the bucket: `images/{organisation_id}/{uuid}.{ext}`.
    pub s3_key: String,
    /// Bucket the object was stored in (`S3_BUCKET` at upload time).
    pub s3_bucket: String,
    /// Original file name from the multipart upload.
    pub file_name: String,
    /// MIME type: image/jpeg, image/png, image/webp or image/gif.
    pub content_type: String,
    /// Size in bytes (max 10 MB).
    pub file_size: i64,
    /// Width in pixels; not computed yet, always null.
    pub width: Option<i32>,
    /// Height in pixels; not computed yet, always null.
    pub height: Option<i32>,
    /// Whether this is the organisation's main photo. `set-primary` clears the flag on the other images; an upload with the flag set does not.
    pub is_primary: bool,
}
