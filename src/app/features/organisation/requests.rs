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
