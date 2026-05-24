use super::entities::CountryConnection;
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait CountryConnectionPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, item: CountryConnection) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<CountryConnection>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryConnectionContent {
    pub id: Uuid,
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

impl From<CountryConnection> for CountryConnectionContent {
    fn from(connection: CountryConnection) -> Self {
        Self {
            id: connection.id,
            embassy_org_id: connection.embassy_org_id,
            consulate_org_id: connection.consulate_org_id,
            common_info: connection.common_info,
            location_country_id: connection.location_country_id,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleCountryConnectionsResponse {
    pub items: Vec<CountryConnectionContent>,
    pub total: i64,
}

impl From<Vec<CountryConnection>> for MultipleCountryConnectionsResponse {
    fn from(items: Vec<CountryConnection>) -> Self {
        let response_items: Vec<CountryConnectionContent> = items
            .into_iter()
            .map(CountryConnectionContent::from)
            .collect();
        let total = response_items.len() as i64;

        Self {
            items: response_items,
            total,
        }
    }
}

#[derive(Clone)]
pub struct CountryConnectionPresenterImpl {}
impl CountryConnectionPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl CountryConnectionPresenter for CountryConnectionPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, item: CountryConnection) -> HttpResponse {
        let response_content = CountryConnectionContent::from(item);

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_json(&self, items: Vec<CountryConnection>) -> HttpResponse {
        let response_content = MultipleCountryConnectionsResponse::from(items);

        HttpResponse::Ok().json(response_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_country_connection() -> CountryConnection {
        CountryConnection {
            id: Uuid::new_v4(),
            embassy_org_id: Some(Uuid::new_v4()),
            consulate_org_id: Some(Uuid::new_v4()),
            location_country_id: Some(Uuid::new_v4()),
            common_info: Some("Test Info".to_string()),
        }
    }

    #[test]
    fn test_country_connection_content_from_connection() {
        let connection = create_test_country_connection();
        let connection_id = connection.id;

        let content = CountryConnectionContent::from(connection);

        assert_eq!(content.id, connection_id);
        assert!(content.embassy_org_id.is_some());
        assert!(content.consulate_org_id.is_some());
        assert!(content.location_country_id.is_some());
        assert_eq!(content.common_info, Some("Test Info".to_string()));
    }

    #[test]
    fn test_country_connection_content_with_nulls() {
        let connection = CountryConnection {
            id: Uuid::new_v4(),
            embassy_org_id: None,
            consulate_org_id: None,
            location_country_id: None,
            common_info: None,
        };

        let content = CountryConnectionContent::from(connection);

        assert!(content.embassy_org_id.is_none());
        assert!(content.consulate_org_id.is_none());
        assert!(content.location_country_id.is_none());
        assert!(content.common_info.is_none());
    }

    #[test]
    fn test_multiple_country_connections_response_from_vec() {
        let conn1 = create_test_country_connection();
        let conn2 = create_test_country_connection();
        let connections = vec![conn1, conn2];

        let response = MultipleCountryConnectionsResponse::from(connections);

        assert_eq!(response.total, 2);
        assert_eq!(response.items.len(), 2);
    }

    #[test]
    fn test_multiple_country_connections_response_empty() {
        let connections: Vec<CountryConnection> = vec![];

        let response = MultipleCountryConnectionsResponse::from(connections);

        assert_eq!(response.total, 0);
        assert_eq!(response.items.len(), 0);
    }

    #[test]
    fn test_country_connection_presenter_new() {
        let presenter = CountryConnectionPresenterImpl::new();
        assert!(std::mem::size_of_val(&presenter) == 0);
    }

    #[test]
    fn test_country_connection_presenter_to_http_res() {
        let presenter = CountryConnectionPresenterImpl::new();
        let response = presenter.to_http_res();

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_country_connection_presenter_to_single_json() {
        let presenter = CountryConnectionPresenterImpl::new();
        let connection = create_test_country_connection();

        let response = presenter.to_single_json(connection);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_country_connection_presenter_to_multi_json() {
        let presenter = CountryConnectionPresenterImpl::new();
        let conn1 = create_test_country_connection();
        let conn2 = create_test_country_connection();
        let connections = vec![conn1, conn2];

        let response = presenter.to_multi_json(connections);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_country_connection_presenter_clone() {
        let presenter = CountryConnectionPresenterImpl::new();
        let _cloned = presenter.clone();
    }
}
