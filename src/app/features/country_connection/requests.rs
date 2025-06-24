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
