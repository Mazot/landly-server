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

/// Weekday keys of `opening_hours`, Monday first (the order `compute_open_now` reads them in).
pub const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Whether `text` is a `"HH:MM"` time of day; `"24:00"` only as a closing time.
fn is_time(text: &str, closing: bool) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' || !text.is_char_boundary(2) {
        return false;
    }
    let (Ok(hours), Ok(minutes)) = (text[..2].parse::<u8>(), text[3..].parse::<u8>()) else {
        return false;
    };
    if !bytes[..2].iter().chain(&bytes[3..]).all(u8::is_ascii_digit) {
        return false;
    }
    (hours < 24 && minutes < 60) || (closing && hours == 24 && minutes == 0)
}

/// `opening_hours` as `compute_open_now` reads it: `{"mon": [["09:00", "13:00"], ...], ...}` —
/// weekday keys only, each a list of `[open, close]` `"HH:MM"` pairs with open before close
/// (`"24:00"` closes at midnight). A day that is missing or empty is closed.
pub fn opening_hours(field: &str, value: Option<&Value>) -> Result<(), AppError> {
    let Some(days) = value.and_then(Value::as_object) else {
        return Ok(()); // absence and non-objects are `json_object`'s concern
    };
    for (day, intervals) in days {
        if !WEEKDAYS.contains(&day.as_str()) {
            return Err(unprocessable(format!(
                "{field}: unknown day \"{day}\" (expected one of {})",
                WEEKDAYS.join(", ")
            )));
        }
        let Some(intervals) = intervals.as_array() else {
            return Err(unprocessable(format!(
                "{field}.{day} must be a list of [open, close] pairs"
            )));
        };
        for interval in intervals {
            let pair = interval.as_array().map(|pair| {
                (
                    pair.first().and_then(Value::as_str),
                    pair.get(1).and_then(Value::as_str),
                    pair.len(),
                )
            });
            let Some((Some(open), Some(close), 2)) = pair else {
                return Err(unprocessable(format!(
                    "{field}.{day} must be a list of [open, close] pairs"
                )));
            };
            if !is_time(open, false) || !is_time(close, true) || open >= close {
                return Err(unprocessable(format!(
                    "{field}.{day}: \"{open}\"–\"{close}\" must be HH:MM times with the opening first"
                )));
            }
        }
    }
    Ok(())
}

/// An IANA time zone name chrono-tz knows (e.g. "Europe/Berlin").
pub fn time_zone(field: &str, value: Option<&str>) -> Result<(), AppError> {
    match value {
        Some(name) if name.parse::<chrono_tz::Tz>().is_err() => Err(unprocessable(format!(
            "{field} must be an IANA time zone such as Europe/Berlin"
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opening_hours_shape() {
        let ok = json!({ "mon": [["09:00", "13:00"], ["14:00", "24:00"]], "sun": [] });
        assert!(opening_hours("opening_hours", Some(&ok)).is_ok());
        assert!(opening_hours("opening_hours", None).is_ok());
        for bad in [
            json!({ "monday": [] }),
            json!({ "mon": "09:00-18:00" }),
            json!({ "mon": [["09:00"]] }),
            json!({ "mon": [["09:00", "18:00", "20:00"]] }),
            json!({ "mon": [["9:00", "18:00"]] }),
            json!({ "mon": [["18:00", "09:00"]] }),
            json!({ "mon": [["24:00", "24:00"]] }),
            json!({ "mon": [["09:60", "18:00"]] }),
            json!({ "mon": [["0a:00", "18:00"]] }),
            json!({ "mon": [[9, 18]] }),
        ] {
            assert!(is_422(opening_hours("opening_hours", Some(&bad))), "{bad}");
        }
    }

    #[test]
    fn test_time_zone_must_be_iana() {
        assert!(time_zone("timezone", Some("Europe/Belgrade")).is_ok());
        assert!(time_zone("timezone", None).is_ok());
        assert!(is_422(time_zone("timezone", Some("Mars/Olympus"))));
        assert!(is_422(time_zone("timezone", Some("GMT+3 Moscow"))));
    }

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
