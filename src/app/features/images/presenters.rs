use super::entities::Image;
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait ImagePresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, item: Image, url: String) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<Image>, urls: Vec<String>, total: i64) -> HttpResponse;
}

/// Response DTO for a single image.
///
/// The `url` field is the publicly-accessible URL and is computed by the
/// usecase via the injected `StorageService` — it is never stored in the DB.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Image ID.
    pub id: Uuid,
    /// Organisation the photo belongs to.
    pub organisation_id: Uuid,
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
    /// Publicly-accessible URL for this image.
    pub url: String,
    /// Upload time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Last change, e.g. primary flag toggled (UTC, no timezone).
    pub updated_at: NaiveDateTime,
}

impl ImageContent {
    pub fn from_image_with_url(image: Image, url: String) -> Self {
        Self {
            id: image.id,
            organisation_id: image.organisation_id,
            file_name: image.file_name,
            content_type: image.content_type,
            file_size: image.file_size,
            width: image.width,
            height: image.height,
            is_primary: image.is_primary,
            url,
            created_at: image.created_at,
            updated_at: image.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MultipleImagesResponse {
    /// Images of the organisation, newest first.
    pub items: Vec<ImageContent>,
    /// Number of items returned.
    pub total: i64,
}

#[derive(Clone)]
pub struct ImagePresenterImpl {}

impl ImagePresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImagePresenter for ImagePresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, item: Image, url: String) -> HttpResponse {
        let content = ImageContent::from_image_with_url(item, url);
        HttpResponse::Ok().json(content)
    }

    fn to_multi_json(&self, items: Vec<Image>, urls: Vec<String>, total: i64) -> HttpResponse {
        let response_items: Vec<ImageContent> = items
            .into_iter()
            .zip(urls)
            .map(|(img, url)| ImageContent::from_image_with_url(img, url))
            .collect();

        HttpResponse::Ok().json(MultipleImagesResponse {
            items: response_items,
            total,
        })
    }
}
