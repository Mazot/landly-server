use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOrganisationRequest {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub founder_country_id: Option<Uuid>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    /// Structured hours by weekday, e.g. {"mon": [["09:00","13:00"]], ...}
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone name used to compute `openNow`, e.g. "Europe/Berlin"
    pub timezone: Option<String>,
    pub cost: Option<String>,
    pub added_by: Option<String>,
    pub google_place_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateOrganisationRequest {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub founder_country_id: Option<Uuid>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
}

/// Map search (design: map.jsx + map-filters.jsx). Either a bbox
/// (min_lat/min_lng/max_lat/max_lng) or an origin point with radius_km.
#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct SearchOrganisationsQueryRequest {
    pub min_lat: Option<f64>,
    pub min_lng: Option<f64>,
    pub max_lat: Option<f64>,
    pub max_lng: Option<f64>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub radius_km: Option<f64>,
    /// Comma-separated org type slugs, e.g. "embassy,community"
    pub types: Option<String>,
    pub open_now: Option<bool>,
    /// Comma-separated language names, e.g. "Russian,English"
    pub languages: Option<String>,
    pub verified: Option<bool>,
    pub min_rating: Option<f64>,
    pub added_by: Option<String>,
    pub cost: Option<String>,
    /// One of: nearest (default when origin given), recent, verified
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct OrganisationsListQueryRequest {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub founder_country_id: Option<Uuid>,
    pub limit: Option<i64>,
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
