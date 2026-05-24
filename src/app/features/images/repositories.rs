use super::entities::{CreateImage, Image};
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
}

// ---------------------------------------------------------------------------
// Repository input types
// ---------------------------------------------------------------------------

pub struct CreateImageRepositoryInput {
    pub organisation_id: Uuid,
    pub uploaded_by: Uuid,
    pub s3_key: String,
    pub s3_bucket: String,
    pub file_name: String,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub is_primary: bool,
}
