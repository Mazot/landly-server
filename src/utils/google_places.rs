//! Google Places API (New) — server-side lookups of a `place_id`.
//!
//! Google's terms allow storing only the place ID indefinitely; coordinates
//! fetched here may be cached and must be refreshed (< 30 days), and nothing
//! else (rating, hours, photos) is stored — the client fetches those live.
//! Configured by `GOOGLE_PLACES_API_KEY` (a server key restricted to the
//! Places API and the server's IPs); without it lookups are disabled and the
//! organisation usecase trusts the client's coordinates (see there).

use crate::error::AppError;
use serde::Deserialize;
use serde_json::json;
use std::{future::Future, pin::Pin, time::Duration};

pub type PlaceFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PlaceDetails, AppError>> + Send + 'a>>;

/// What the server keeps (or checks) from a Google place.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceDetails {
    /// Latitude in degrees.
    pub latitude: f64,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Google's `businessStatus` (`OPERATIONAL`, `CLOSED_TEMPORARILY`,
    /// `CLOSED_PERMANENTLY`); a moderation signal only, never stored.
    pub business_status: Option<String>,
}

/// Object-safe so usecases can hold `Arc<dyn PlaceLookup>` and tests a stub.
pub trait PlaceLookup: Send + Sync + 'static {
    /// Coordinates of `place_id`. `UnprocessableEntity` when Google does not
    /// know the place, `ServiceUnavailable` when Google cannot be reached.
    fn lookup<'a>(&'a self, place_id: &'a str) -> PlaceFuture<'a>;
}

/// Place IDs are URL-safe tokens; anything else is rejected before it is
/// put into the request path.
pub fn validate_place_id(place_id: &str) -> Result<(), AppError> {
    let ok = !place_id.is_empty()
        && place_id.len() <= 512
        && place_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(AppError::UnprocessableEntity(
            json!({ "error": "google_place_id is not a valid Google place ID" }),
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaceResponse {
    location: Option<LatLng>,
    business_status: Option<String>,
}

#[derive(Deserialize)]
struct LatLng {
    latitude: f64,
    longitude: f64,
}

/// Maps a Place Details response (HTTP status + body) to the result. Pure, so
/// the whole mapping is unit-tested without network.
pub fn parse_place_details(status: u16, body: &str) -> Result<PlaceDetails, AppError> {
    match status {
        200 => {
            let place: PlaceResponse = serde_json::from_str(body).map_err(|e| {
                log::error!("Unexpected Places API response: {e}");
                unavailable()
            })?;
            let location = place.location.ok_or_else(|| {
                AppError::UnprocessableEntity(
                    json!({ "error": "This Google place has no location" }),
                )
            })?;
            Ok(PlaceDetails {
                latitude: location.latitude,
                longitude: location.longitude,
                business_status: place.business_status,
            })
        }
        // Google also answers 400 for a bad server key (reason API_KEY_INVALID,
        // API_KEY_IP_ADDRESS_BLOCKED, …): a configuration failure, not the
        // client's place ID.
        400 if has_api_key_error(body) => {
            log::error!("Places API rejected the server key: {body}");
            Err(unavailable())
        }
        // Unknown or malformed place ID.
        400 | 404 => Err(AppError::UnprocessableEntity(
            json!({ "error": "Unknown google_place_id" }),
        )),
        // Bad key, quota, outage — not the client's fault.
        other => {
            log::error!("Places API answered {other}: {body}");
            Err(unavailable())
        }
    }
}

/// Google API errors carry `error.details[]`; `google.rpc.ErrorInfo` entries
/// have a machine-readable `reason`. Every key problem starts with `API_KEY_`.
fn has_api_key_error(body: &str) -> bool {
    #[derive(Deserialize)]
    struct Envelope {
        error: Option<ErrorBody>,
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        #[serde(default)]
        details: Vec<serde_json::Value>,
    }

    serde_json::from_str::<Envelope>(body)
        .ok()
        .and_then(|envelope| envelope.error)
        .is_some_and(|error| {
            error.details.iter().any(|detail| {
                detail["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.starts_with("API_KEY_"))
            })
        })
}

fn unavailable() -> AppError {
    AppError::ServiceUnavailable(
        json!({ "error": "Google Places is unavailable, try again later" }),
    )
}

#[derive(Clone)]
pub struct GooglePlacesClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GooglePlacesClient {
    const BASE_URL: &'static str = "https://places.googleapis.com/v1";
    /// Only the fields we use: they decide the SKU Google bills.
    const FIELD_MASK: &'static str = "id,location,businessStatus";

    /// `Some` when `GOOGLE_PLACES_API_KEY` is set (and non-empty).
    pub fn try_from_env() -> Option<Self> {
        let api_key = std::env::var(crate::constants::env_key::GOOGLE_PLACES_API_KEY)
            .ok()
            .filter(|k| !k.is_empty())?;
        Some(Self::new(api_key, Self::BASE_URL))
    }

    pub fn new(api_key: String, base_url: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client with a timeout");
        Self {
            http,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

impl PlaceLookup for GooglePlacesClient {
    fn lookup<'a>(&'a self, place_id: &'a str) -> PlaceFuture<'a> {
        Box::pin(async move {
            validate_place_id(place_id)?;
            let response = self
                .http
                .get(format!("{}/places/{place_id}", self.base_url))
                .header("X-Goog-Api-Key", &self.api_key)
                .header("X-Goog-FieldMask", Self::FIELD_MASK)
                .send()
                .await
                .map_err(|e| {
                    log::error!("Places API request failed: {e}");
                    unavailable()
                })?;
            let status = response.status().as_u16();
            let body = response.text().await.map_err(|_| unavailable())?;

            parse_place_details(status, &body)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_place_details_ok() {
        let body = r#"{"id":"ChIJ","location":{"latitude":52.5163,"longitude":13.3777},"businessStatus":"OPERATIONAL"}"#;
        assert_eq!(
            parse_place_details(200, body).unwrap(),
            PlaceDetails {
                latitude: 52.5163,
                longitude: 13.3777,
                business_status: Some("OPERATIONAL".into())
            }
        );
    }

    #[test]
    fn test_parse_place_details_maps_errors() {
        for status in [400, 404] {
            assert!(matches!(
                parse_place_details(status, "{}"),
                Err(AppError::UnprocessableEntity(_))
            ));
        }
        for status in [403, 429, 500, 503] {
            assert!(matches!(
                parse_place_details(status, "{}"),
                Err(AppError::ServiceUnavailable(_))
            ));
        }
        assert!(matches!(
            parse_place_details(200, "not json"),
            Err(AppError::ServiceUnavailable(_))
        ));
        assert!(matches!(
            parse_place_details(200, r#"{"id":"x"}"#),
            Err(AppError::UnprocessableEntity(_))
        ));
    }

    /// Review regression: a bad server key is a 400 from Google, but it must
    /// surface as 503 (our configuration), not as "Unknown google_place_id".
    #[test]
    fn test_parse_place_details_treats_api_key_errors_as_unavailable() {
        let invalid_key = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_INVALID","domain":"googleapis.com","metadata":{"service":"places.googleapis.com"}}]}}"#;
        assert!(matches!(
            parse_place_details(400, invalid_key),
            Err(AppError::ServiceUnavailable(_))
        ));
        let ip_blocked = invalid_key.replace("API_KEY_INVALID", "API_KEY_IP_ADDRESS_BLOCKED");
        assert!(matches!(
            parse_place_details(400, &ip_blocked),
            Err(AppError::ServiceUnavailable(_))
        ));

        let bad_place = r#"{"error":{"code":400,"message":"Not a valid Place ID.","status":"INVALID_ARGUMENT"}}"#;
        assert!(matches!(
            parse_place_details(400, bad_place),
            Err(AppError::UnprocessableEntity(_))
        ));
        let other_reason = invalid_key.replace("API_KEY_INVALID", "SOMETHING_ELSE");
        assert!(matches!(
            parse_place_details(400, &other_reason),
            Err(AppError::UnprocessableEntity(_))
        ));
    }

    #[test]
    fn test_validate_place_id() {
        assert!(validate_place_id("ChIJN1t_tDeuEmsRUsoyG83frY4").is_ok());
        for bad in [
            "",
            "../admin",
            "a b",
            "id?key=x",
            "id/other",
            &"x".repeat(513),
        ] {
            assert!(validate_place_id(bad).is_err(), "{bad:?}");
        }
    }

    /// Network failures (here: a closed port) are 503, never 500.
    #[actix_web::test]
    async fn test_unreachable_places_api_is_unavailable() {
        let client = GooglePlacesClient::new("key".into(), "http://127.0.0.1:1/v1");
        assert!(matches!(
            client.lookup("ChIJabc").await,
            Err(AppError::ServiceUnavailable(_))
        ));
        assert!(matches!(
            client.lookup("../x").await,
            Err(AppError::UnprocessableEntity(_))
        ));
    }
}
