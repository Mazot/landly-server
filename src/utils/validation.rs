//! Input size limits shared by every usecase (security audit M3).
//!
//! Without them any authenticated user could store megabytes per field (the
//! JSON body limit alone was 2 MB), bloating the database and every list or
//! search response built from it. Limits count Unicode characters, not
//! bytes, so Cyrillic or CJK text is not penalised; JSON documents are
//! limited in serialized bytes.
//!
//! The numbers are generous for real content and are meant to be tuned here,
//! in one place.

use crate::error::AppError;
use serde_json::{Value, json};

/// Maximum JSON request body, bytes (`JsonConfig` in `configure_api`).
/// Image uploads are multipart and have their own, larger cap.
pub const MAX_JSON_BODY_BYTES: usize = 64 * 1024;

pub mod limits {
    // Organisations
    pub const ORG_NAME: usize = 200;
    pub const PHONE: usize = 32;
    pub const EMAIL: usize = 254;
    pub const ADDRESS: usize = 300;
    pub const CITY: usize = 100;
    pub const URL: usize = 2048;
    /// Telegram / WhatsApp handle or link.
    pub const HANDLE: usize = 100;
    pub const DESCRIPTION: usize = 2000;
    pub const TIMEZONE: usize = 64;
    /// One service / language / skill tag.
    pub const TAG: usize = 60;
    pub const TAGS_PER_LIST: usize = 30;
    /// Serialized size of `opening_hours` / `notification_settings`.
    pub const JSON_DOCUMENT_BYTES: usize = 4 * 1024;

    // People
    pub const PERSON_NAME: usize = 100;
    pub const BIO: usize = 1000;
    pub const SKILLS: usize = 20;
    pub const LANGUAGE_IDS: usize = 30;
    pub const VOUCH_NOTE: usize = 500;

    // Reviews, reports, check-ins, bookmarks
    pub const REVIEW_TEXT: usize = 2000;
    pub const REVIEW_TOPIC: usize = 100;
    pub const REPORT_REASON: usize = 1000;
    pub const CHECKIN_TIP: usize = 300;
    pub const SAVED_NOTE: usize = 1000;
    pub const SAVED_LIST_NAME: usize = 100;

    // Reference data
    pub const COMMON_INFO: usize = 2000;
    pub const ORG_TYPE_KEY: usize = 50;
    pub const COLOR: usize = 32;
    pub const ORG_TYPE_TITLE: usize = 100;
    pub const SLUG: usize = 64;

    // Query-string filters
    pub const FILTER: usize = 100;
    pub const FILTER_LIST_ITEMS: usize = 20;
}

fn unprocessable(message: String) -> AppError {
    AppError::UnprocessableEntity(json!({ "error": message }))
}

/// At most `max` characters; `None` passes.
pub fn max_chars(field: &str, value: Option<&str>, max: usize) -> Result<(), AppError> {
    match value {
        Some(v) if v.chars().count() > max => Err(unprocessable(format!(
            "{field} must be at most {max} characters"
        ))),
        _ => Ok(()),
    }
}

/// Same for a field that is always present.
pub fn required_max_chars(field: &str, value: &str, max: usize) -> Result<(), AppError> {
    max_chars(field, Some(value), max)
}

/// At most `max` items; `None` passes.
pub fn max_items<T>(field: &str, items: Option<&[T]>, max: usize) -> Result<(), AppError> {
    match items {
        Some(list) if list.len() > max => Err(unprocessable(format!(
            "{field} must have at most {max} items"
        ))),
        _ => Ok(()),
    }
}

/// A list of tags: at most `max_items` entries of at most `max_chars` each.
pub fn tag_list(
    field: &str,
    items: Option<&[String]>,
    max_items_count: usize,
    max_tag_chars: usize,
) -> Result<(), AppError> {
    max_items(field, items, max_items_count)?;
    for item in items.unwrap_or_default() {
        max_chars(field, Some(item), max_tag_chars)?;
    }
    Ok(())
}

/// A JSON document that must be an object of at most `max_bytes` serialized.
pub fn json_object(field: &str, value: Option<&Value>, max_bytes: usize) -> Result<(), AppError> {
    let Some(value) = value else { return Ok(()) };

    if !value.is_object() {
        return Err(unprocessable(format!("{field} must be a JSON object")));
    }
    // Serializing a `Value` cannot fail.
    if value.to_string().len() > max_bytes {
        return Err(unprocessable(format!(
            "{field} must be at most {max_bytes} bytes of JSON"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_422<T>(result: Result<T, AppError>) -> bool {
        matches!(result, Err(AppError::UnprocessableEntity(_)))
    }

    #[test]
    fn test_max_chars_counts_characters_not_bytes() {
        assert!(max_chars("f", Some("ab"), 2).is_ok());
        assert!(is_422(max_chars("f", Some("abc"), 2)));
        assert!(max_chars("f", None, 0).is_ok());
        // 3 Cyrillic letters = 6 bytes but 3 characters.
        assert!(max_chars("f", Some("при"), 3).is_ok());
        assert!(is_422(max_chars("f", Some("прив"), 3)));
        assert!(is_422(required_max_chars("f", "abc", 2)));
    }

    #[test]
    fn test_error_names_the_field_and_the_limit() {
        let Err(AppError::UnprocessableEntity(body)) = max_chars("description", Some("xyz"), 2)
        else {
            panic!("expected 422");
        };
        assert_eq!(body["error"], "description must be at most 2 characters");
    }

    #[test]
    fn test_tag_list_limits_count_and_length() {
        let tags = |n: usize, len: usize| vec!["x".repeat(len); n];
        assert!(tag_list("services", Some(&tags(3, 5)), 3, 5).is_ok());
        assert!(is_422(tag_list("services", Some(&tags(4, 5)), 3, 5)));
        assert!(is_422(tag_list("services", Some(&tags(1, 6)), 3, 5)));
        assert!(tag_list("services", None, 0, 0).is_ok());
        assert!(max_items::<u8>("ids", Some(&[1, 2]), 2).is_ok());
        assert!(is_422(max_items::<u8>("ids", Some(&[1, 2, 3]), 2)));
    }

    #[test]
    fn test_json_object_must_be_a_small_object() {
        assert!(json_object("hours", None, 10).is_ok());
        assert!(json_object("hours", Some(&json!({ "mon": [] })), 100).is_ok());
        assert!(is_422(json_object("hours", Some(&json!([1, 2])), 100)));
        assert!(is_422(json_object("hours", Some(&json!("text")), 100)));
        assert!(is_422(json_object(
            "hours",
            Some(&json!({ "mon": "x".repeat(200) })),
            100
        )));
    }
}
