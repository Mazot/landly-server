use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOrganisationRequest {
    /// Display name.
    pub name: String,
    /// Contact phone, free text (format is only sanity-checked into moderation flags).
    pub tel: Option<String>,
    /// Contact email.
    pub email: Option<String>,
    /// Street address, free text.
    pub address: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Country the organisation is located in.
    pub location_country_id: Option<Uuid>,
    /// Organisation type (embassy, business, helper, community, volunteer, …).
    pub organisation_type_id: Option<Uuid>,
    /// Latitude in degrees (-90..90).
    pub latitude: Option<f64>,
    /// Longitude in degrees (-180..180).
    pub longitude: Option<f64>,
    /// Home country of the founders / community it serves (the corridor's "from" country).
    pub founder_country_id: Option<Uuid>,
    /// City name.
    pub city: Option<String>,
    /// Website URL.
    pub website: Option<String>,
    /// Telegram handle or link.
    pub telegram: Option<String>,
    /// WhatsApp number or link.
    pub whatsapp: Option<String>,
    /// Services offered, free-text tags. Defaults to empty.
    pub services: Option<Vec<String>>,
    /// Languages spoken, by name (e.g. "Russian"). Defaults to empty.
    pub languages: Option<Vec<String>>,
    /// Structured hours by weekday, e.g. {"mon": [["09:00","13:00"]], ...}
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone name used to compute `openNow`, e.g. "Europe/Berlin"
    pub timezone: Option<String>,
    /// `free` or `paid`.
    pub cost: Option<String>,
    /// Who added it: `official`, `community` or `volunteer`.
    pub added_by: Option<String>,
    /// Google Places ID when imported from Google.
    pub google_place_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateOrganisationRequest {
    /// Display name; omit to keep the current value.
    pub name: Option<String>,
    /// Contact phone, free text (format is only sanity-checked into moderation flags); omit to keep the current value.
    pub tel: Option<String>,
    /// Contact email; omit to keep the current value.
    pub email: Option<String>,
    /// Street address, free text; omit to keep the current value.
    pub address: Option<String>,
    /// Free-text description; omit to keep the current value.
    pub description: Option<String>,
    /// Country the organisation is located in; omit to keep the current value.
    pub location_country_id: Option<Uuid>,
    /// Organisation type (embassy, business, helper, community, volunteer, …); omit to keep the current value.
    pub organisation_type_id: Option<Uuid>,
    /// Latitude in degrees (-90..90); omit to keep the current value.
    pub latitude: Option<f64>,
    /// Longitude in degrees (-180..180); omit to keep the current value.
    pub longitude: Option<f64>,
    /// Home country of the founders / community it serves (the corridor's "from" country); omit to keep the current value.
    pub founder_country_id: Option<Uuid>,
    /// City name; omit to keep the current value.
    pub city: Option<String>,
    /// Website URL; omit to keep the current value.
    pub website: Option<String>,
    /// Telegram handle or link; omit to keep the current value.
    pub telegram: Option<String>,
    /// WhatsApp number or link; omit to keep the current value.
    pub whatsapp: Option<String>,
    /// Services offered, free-text tags; omit to keep the current value.
    pub services: Option<Vec<String>>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter; omit to keep the current value.
    pub languages: Option<Vec<String>>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun); omit to keep the current value.
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`; omit to keep the current value.
    pub timezone: Option<String>,
    /// `free` or `paid`; omit to keep the current value.
    pub cost: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CheckinRequest {
    /// "The place is still active" confirmation; defaults to true
    pub still_active: Option<bool>,
    /// Optional community tip shown on the detail page
    pub tip: Option<String>,
}

/// Map search (design: map.jsx + map-filters.jsx). Either a bbox
/// (min_lat/min_lng/max_lat/max_lng) or an origin point with radius_km.
#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct SearchOrganisationsQueryRequest {
    /// Bounding box south edge (-90..90). Give all four bbox values, or an origin.
    pub min_lat: Option<f64>,
    /// Bounding box west edge (-180..180).
    pub min_lng: Option<f64>,
    /// Bounding box north edge (-90..90).
    pub max_lat: Option<f64>,
    /// Bounding box east edge (-180..180).
    pub max_lng: Option<f64>,
    /// Origin latitude; enables `distanceKm` and `sort=nearest`.
    pub lat: Option<f64>,
    /// Origin longitude.
    pub lng: Option<f64>,
    /// Radius around the origin in km.
    pub radius_km: Option<f64>,
    /// Comma-separated org type slugs, e.g. "embassy,community"
    pub types: Option<String>,
    /// Only places open right now. Default false.
    pub open_now: Option<bool>,
    /// Comma-separated language names, e.g. "Russian,English"
    pub languages: Option<String>,
    /// Only verified places. Default false.
    pub verified: Option<bool>,
    /// Minimum average review rating (1–5).
    pub min_rating: Option<f64>,
    /// `official`, `community` or `volunteer`.
    pub added_by: Option<String>,
    /// `free` or `paid`.
    pub cost: Option<String>,
    /// One of: nearest (default when origin given), recent, verified
    pub sort: Option<String>,
    /// Page size (default 50, max 200). At most 500 rows are scanned per search.
    pub limit: Option<i64>,
    /// Rows to skip (default 0).
    pub offset: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct OrganisationsListQueryRequest {
    /// Case-insensitive substring filter on the name.
    pub name: Option<String>,
    /// Case-insensitive substring filter on the phone.
    pub tel: Option<String>,
    /// Case-insensitive substring filter on the email.
    pub email: Option<String>,
    /// Case-insensitive substring filter on the address.
    pub address: Option<String>,
    /// Only organisations located in this country.
    pub location_country_id: Option<Uuid>,
    /// Only this organisation type.
    pub organisation_type_id: Option<Uuid>,
    /// Only organisations founded by people from this country.
    pub founder_country_id: Option<Uuid>,
    /// Page size (default 20, max 100).
    pub limit: Option<i64>,
    /// Rows to skip (default 0, max 150).
    pub offset: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_organisation_request_serialization() {
        let request = CreateOrganisationRequest {
            name: "Test Org".to_string(),
            tel: Some("+1234567890".to_string()),
            email: Some("test@example.com".to_string()),
            address: Some("123 Test St".to_string()),
            description: Some("Test Description".to_string()),
            location_country_id: Some(Uuid::new_v4()),
            organisation_type_id: Some(Uuid::new_v4()),
            latitude: Some(40.7128),
            longitude: Some(-74.0060),
            founder_country_id: Some(Uuid::new_v4()),
            city: Some("Berlin".to_string()),
            website: None,
            telegram: None,
            whatsapp: None,
            services: Some(vec!["Notary".to_string()]),
            languages: Some(vec!["Russian".to_string(), "German".to_string()]),
            opening_hours: None,
            timezone: Some("Europe/Berlin".to_string()),
            cost: Some("free".to_string()),
            added_by: Some("community".to_string()),
            google_place_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test Org"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_create_organisation_request_deserialization() {
        let json = r#"{
            "name": "Test Org",
            "tel": "+1234567890",
            "email": "test@example.com",
            "address": "123 Test St",
            "description": "Test Description",
            "location_country_id": null,
            "organisation_type_id": null,
            "latitude": 40.7128,
            "longitude": -74.0060,
            "founder_country_id": null
        }"#;

        let request: CreateOrganisationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Test Org");
        assert_eq!(request.tel, Some("+1234567890".to_string()));
        assert_eq!(request.latitude, Some(40.7128));
    }

    #[test]
    fn test_update_organisation_request_all_none() {
        let request = UpdateOrganisationRequest {
            name: None,
            tel: None,
            email: None,
            address: None,
            description: None,
            location_country_id: None,
            organisation_type_id: None,
            latitude: None,
            longitude: None,
            founder_country_id: None,
            city: None,
            website: None,
            telegram: None,
            whatsapp: None,
            services: None,
            languages: None,
            opening_hours: None,
            timezone: None,
            cost: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("null"));
    }

    #[test]
    fn test_update_organisation_request_partial_update() {
        let request = UpdateOrganisationRequest {
            name: Some("Updated Name".to_string()),
            tel: None,
            email: Some("new@example.com".to_string()),
            address: None,
            description: None,
            location_country_id: None,
            organisation_type_id: None,
            latitude: None,
            longitude: None,
            founder_country_id: None,
            city: None,
            website: None,
            telegram: None,
            whatsapp: None,
            services: None,
            languages: None,
            opening_hours: None,
            timezone: None,
            cost: None,
        };

        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.email, Some("new@example.com".to_string()));
        assert!(request.tel.is_none());
    }

    #[test]
    fn test_organisations_list_query_request_empty() {
        let request = OrganisationsListQueryRequest {
            name: None,
            tel: None,
            email: None,
            address: None,
            location_country_id: None,
            organisation_type_id: None,
            founder_country_id: None,
            limit: None,
            offset: None,
        };

        assert!(request.name.is_none());
        assert!(request.limit.is_none());
    }

    #[test]
    fn test_organisations_list_query_request_with_pagination() {
        let request = OrganisationsListQueryRequest {
            name: Some("Test".to_string()),
            tel: None,
            email: None,
            address: None,
            location_country_id: None,
            organisation_type_id: None,
            founder_country_id: None,
            limit: Some(10),
            offset: Some(20),
        };

        assert_eq!(request.limit, Some(10));
        assert_eq!(request.offset, Some(20));
        assert_eq!(request.name, Some("Test".to_string()));
    }

    #[test]
    fn test_organisations_list_query_request_with_filters() {
        let country_id = Uuid::new_v4();
        let org_type_id = Uuid::new_v4();

        let request = OrganisationsListQueryRequest {
            name: Some("Search Term".to_string()),
            tel: Some("+123".to_string()),
            email: Some("test@".to_string()),
            address: Some("Street".to_string()),
            location_country_id: Some(country_id),
            organisation_type_id: Some(org_type_id),
            founder_country_id: Some(country_id),
            limit: Some(50),
            offset: Some(0),
        };

        assert_eq!(request.location_country_id, Some(country_id));
        assert_eq!(request.organisation_type_id, Some(org_type_id));
    }
}
