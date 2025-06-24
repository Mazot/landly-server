use super::entities::Organisation;
use serde::{Deserialize, Serialize};
use actix_web::{
    HttpResponse,
    http::StatusCode
};
use chrono::NaiveDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

pub trait OrganisationPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation>;
    fn to_single_json(&self, item: Organisation) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<Organisation>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationContent {
    pub id: Uuid,
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<Organisation> for OrganisationContent {
    fn from(org: Organisation) -> Self {
        Self {
            id: org.id,
            name: org.name,
            tel: org.tel,
            email: org.email,
            address: org.address,
            description: org.description,
            location_country_id: org.location_country_id,
            organisation_type_id: org.organisation_type_id,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleOrganisationsResponse {
    pub items: Vec<OrganisationContent>,
    pub total: i64,
}

impl From<Vec<Organisation>> for MultipleOrganisationsResponse {
    fn from(items: Vec<Organisation>) -> Self {
        let response_items: Vec<OrganisationContent> = items
            .into_iter()
            .map(OrganisationContent::from)
            .collect();
        let count = response_items.len() as i64;

        Self {
            items: response_items,
            total: count,
        }
    }
}

#[derive(Clone)]
pub struct OrganisationPresenterImpl {}
impl OrganisationPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl OrganisationPresenter for OrganisationPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation> {
        HttpResponse::<Organisation>::with_body(
            StatusCode::OK, item
        )
    }

    fn to_single_json(&self, item: Organisation) -> HttpResponse {
        let response_content = OrganisationContent::from(item);

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_json(&self, items: Vec<Organisation>) -> HttpResponse {
        let response_content = MultipleOrganisationsResponse::from(items);

        HttpResponse::Ok().json(response_content)
    }
}
