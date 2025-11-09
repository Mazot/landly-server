use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCountryConnectionRequest {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema, Debug)]
pub struct UpdateCountryConnectionRequest {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema, IntoParams, Debug)]
pub struct CountryConnectionsListQueryParams {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub location_country_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_country_connection_request_serialization() {
        let request = CreateCountryConnectionRequest {
            embassy_org_id: Some(Uuid::new_v4()),
            consulate_org_id: Some(Uuid::new_v4()),
            common_info: Some("Test Info".to_string()),
            location_country_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test Info"));
    }

    #[test]
    fn test_create_country_connection_request_deserialization() {
        let json = r#"{
            "embassy_org_id": null,
            "consulate_org_id": null,
            "common_info": "Test Common Info",
            "location_country_id": null
        }"#;

        let request: CreateCountryConnectionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.common_info, Some("Test Common Info".to_string()));
        assert!(request.embassy_org_id.is_none());
    }

    #[test]
    fn test_update_country_connection_request_all_none() {
        let request = UpdateCountryConnectionRequest {
            embassy_org_id: None,
            consulate_org_id: None,
            common_info: None,
            location_country_id: None,
        };

        assert!(request.embassy_org_id.is_none());
        assert!(request.consulate_org_id.is_none());
        assert!(request.common_info.is_none());
        assert!(request.location_country_id.is_none());
    }

    #[test]
    fn test_update_country_connection_request_partial() {
        let embassy_id = Uuid::new_v4();
        let request = UpdateCountryConnectionRequest {
            embassy_org_id: Some(embassy_id),
            consulate_org_id: None,
            common_info: Some("Updated Info".to_string()),
            location_country_id: None,
        };

        assert_eq!(request.embassy_org_id, Some(embassy_id));
        assert_eq!(request.common_info, Some("Updated Info".to_string()));
        assert!(request.consulate_org_id.is_none());
    }

    #[test]
    fn test_country_connections_list_query_params_empty() {
        let params = CountryConnectionsListQueryParams {
            embassy_org_id: None,
            consulate_org_id: None,
            location_country_id: None,
            limit: None,
            offset: None,
        };

        assert!(params.embassy_org_id.is_none());
        assert!(params.limit.is_none());
    }

    #[test]
    fn test_country_connections_list_query_params_with_pagination() {
        let params = CountryConnectionsListQueryParams {
            embassy_org_id: None,
            consulate_org_id: None,
            location_country_id: None,
            limit: Some(25),
            offset: Some(50),
        };

        assert_eq!(params.limit, Some(25));
        assert_eq!(params.offset, Some(50));
    }

    #[test]
    fn test_country_connections_list_query_params_with_filters() {
        let embassy_id = Uuid::new_v4();
        let location_id = Uuid::new_v4();
        
        let params = CountryConnectionsListQueryParams {
            embassy_org_id: Some(embassy_id),
            consulate_org_id: None,
            location_country_id: Some(location_id),
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(params.embassy_org_id, Some(embassy_id));
        assert_eq!(params.location_country_id, Some(location_id));
        assert!(params.consulate_org_id.is_none());
    }
}
