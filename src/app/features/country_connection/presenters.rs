use super::entities::CountryConnection;
use serde::{Deserialize, Serialize};
use actix_web::HttpResponse;
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
            total: total,
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
