use super::entities::{ConnectedOrganisation, CountryConnection};
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub trait CountryConnectionPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    /// `orgs`: name and city of the linked places on the map (by id).
    fn to_single_json(
        &self,
        item: CountryConnection,
        orgs: &HashMap<Uuid, ConnectedOrganisation>,
    ) -> HttpResponse;
    fn to_multi_json(
        &self,
        items: Vec<CountryConnection>,
        orgs: &HashMap<Uuid, ConnectedOrganisation>,
    ) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryConnectionContent {
    /// Country connection ID.
    pub id: Uuid,
    /// Organisation acting as the embassy for this connection.
    pub embassy_org_id: Option<Uuid>,
    /// Organisation acting as the consulate for this connection.
    pub consulate_org_id: Option<Uuid>,
    /// Free-text notes about consular services for this connection.
    pub common_info: Option<String>,
    /// Country the embassy/consulate is located in.
    pub location_country_id: Option<Uuid>,
    /// Name and city of the embassy; null without one, or when it is not on the map.
    pub embassy: Option<ConnectedOrganisationContent>,
    /// Name and city of the consulate; null without one, or when it is not on the map.
    pub consulate: Option<ConnectedOrganisationContent>,
}

/// A linked embassy or consulate as the consular-network list shows it.
#[derive(Deserialize, Serialize, ToSchema, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedOrganisationContent {
    /// Organisation ID (opens its page).
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// City, when known.
    pub city: Option<String>,
}

impl From<ConnectedOrganisation> for ConnectedOrganisationContent {
    fn from(org: ConnectedOrganisation) -> Self {
        Self {
            id: org.id,
            name: org.name,
            city: org.city,
        }
    }
}

impl CountryConnectionContent {
    pub fn new(connection: CountryConnection, orgs: &HashMap<Uuid, ConnectedOrganisation>) -> Self {
        let linked = |id: Option<Uuid>| {
            id.and_then(|id| orgs.get(&id).cloned())
                .map(ConnectedOrganisationContent::from)
        };
        Self {
            embassy: linked(connection.embassy_org_id),
            consulate: linked(connection.consulate_org_id),
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
    /// Connections matching the filters.
    pub items: Vec<CountryConnectionContent>,
    /// Number of items returned.
    pub total: i64,
}

impl MultipleCountryConnectionsResponse {
    pub fn new(items: Vec<CountryConnection>, orgs: &HashMap<Uuid, ConnectedOrganisation>) -> Self {
        let response_items: Vec<CountryConnectionContent> = items
            .into_iter()
            .map(|item| CountryConnectionContent::new(item, orgs))
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

    fn to_single_json(
        &self,
        item: CountryConnection,
        orgs: &HashMap<Uuid, ConnectedOrganisation>,
    ) -> HttpResponse {
        let response_content = CountryConnectionContent::new(item, orgs);

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_json(
        &self,
        items: Vec<CountryConnection>,
        orgs: &HashMap<Uuid, ConnectedOrganisation>,
    ) -> HttpResponse {
        let response_content = MultipleCountryConnectionsResponse::new(items, orgs);

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

        let content = CountryConnectionContent::new(connection, &HashMap::new());

        assert_eq!(content.id, connection_id);
        assert!(content.embassy_org_id.is_some());
        assert!(content.consulate_org_id.is_some());
        assert!(content.location_country_id.is_some());
        assert_eq!(content.common_info, Some("Test Info".to_string()));
    }

    /// Linked places come by name and city; one that is not on the map stays null.
    #[test]
    fn test_country_connection_content_names_the_linked_places_on_the_map() {
        let connection = create_test_country_connection();
        let embassy = ConnectedOrganisation {
            id: connection.embassy_org_id.unwrap(),
            name: "Embassy of Russia".to_string(),
            city: Some("Berlin".to_string()),
        };
        let orgs = HashMap::from([(embassy.id, embassy.clone())]);

        let content = CountryConnectionContent::new(connection, &orgs);

        assert_eq!(
            content.embassy,
            Some(ConnectedOrganisationContent {
                id: embassy.id,
                name: "Embassy of Russia".to_string(),
                city: Some("Berlin".to_string()),
            })
        );
        assert!(content.consulate.is_none(), "not on the map: not shown");
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

        let content = CountryConnectionContent::new(connection, &HashMap::new());

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

        let response = MultipleCountryConnectionsResponse::new(connections, &HashMap::new());

        assert_eq!(response.total, 2);
        assert_eq!(response.items.len(), 2);
    }

    #[test]
    fn test_multiple_country_connections_response_empty() {
        let connections: Vec<CountryConnection> = vec![];

        let response = MultipleCountryConnectionsResponse::new(connections, &HashMap::new());

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

        let response = presenter.to_single_json(connection, &HashMap::new());

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_country_connection_presenter_to_multi_json() {
        let presenter = CountryConnectionPresenterImpl::new();
        let conn1 = create_test_country_connection();
        let conn2 = create_test_country_connection();
        let connections = vec![conn1, conn2];

        let response = presenter.to_multi_json(connections, &HashMap::new());

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_country_connection_presenter_clone() {
        let presenter = CountryConnectionPresenterImpl::new();
        let _cloned = presenter.clone();
    }
}
