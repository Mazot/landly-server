use crate::data::schema::images;
use crate::error::AppError;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Selectable, Clone)]
#[diesel(table_name = images)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Image {
    /// Image ID.
    pub id: Uuid,
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
    /// Upload time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Last change, e.g. primary flag toggled (UTC, no timezone).
    pub updated_at: NaiveDateTime,
}

impl Image {
    pub fn create(conn: &mut PgConnection, record: &CreateImage) -> Result<Self, AppError> {
        let result = diesel::insert_into(images::table)
            .values(record)
            .get_result::<Image>(conn)?;

        Ok(result)
    }

    /// Delete an image by ID and return the deleted record (so callers can
    /// retrieve the `s3_key` for subsequent storage cleanup).
    pub fn delete(conn: &mut PgConnection, image_id: Uuid) -> Result<Self, AppError> {
        let result = diesel::delete(images::table.find(image_id)).get_result::<Image>(conn)?;

        Ok(result)
    }

    pub fn fetch_by_id(conn: &mut PgConnection, image_id: Uuid) -> Result<Self, AppError> {
        let result = images::table.find(image_id).get_result::<Image>(conn)?;

        Ok(result)
    }

    pub fn fetch_by_organisation(
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let result = images::table
            .filter(images::organisation_id.eq(org_id))
            .order(images::created_at.desc())
            .load::<Image>(conn)?;

        Ok(result)
    }

    /// Mark `image_id` as the primary image for its organisation.
    ///
    /// Runs inside a transaction:
    /// 1. Clears `is_primary` for every image in the organisation.
    /// 2. Sets `is_primary = true` on the target image.
    ///
    /// The `updated_at` column is managed by the `diesel_manage_updated_at`
    /// trigger defined in the migration, so we do not set it manually.
    pub fn set_primary(
        conn: &mut PgConnection,
        image_id: Uuid,
        org_id: Uuid,
    ) -> Result<Self, AppError> {
        conn.transaction(|conn| {
            // Unset all primaries in the organisation.
            diesel::update(images::table.filter(images::organisation_id.eq(org_id)))
                .set(images::is_primary.eq(false))
                .execute(conn)?;

            // Set the chosen image as primary.
            let result = diesel::update(images::table.find(image_id))
                .set(images::is_primary.eq(true))
                .get_result::<Image>(conn)?;

            Ok(result)
        })
    }
}

// ---------------------------------------------------------------------------
// Insertable DTO — omits auto-generated columns (id, created_at, updated_at).
// ---------------------------------------------------------------------------

#[derive(Debug, Insertable, Clone)]
#[diesel(table_name = images)]
pub struct CreateImage {
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
