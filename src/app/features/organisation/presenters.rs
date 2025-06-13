use super::entities::Organisation;
use crate::data::models::{Country, OrganisationType};
use serde::{Deserialize, Serialize};
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use uuid::Uuid;

pub trait OrganisationPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation>;
    fn to_single_json(&self, item: Organisation) -> HttpResponse;
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrganisationContent {
    pub id: Uuid,
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    // TODO: Change to InnerCountry
    pub location_country: Option<Country>,
    // TODO: Change to InnerOrganisationType
    pub organisation_type: Option<OrganisationType>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<(Organisation, Option<Country>, Option<OrganisationType>)> for OrganisationContent {
    fn from((org, country, org_type): (Organisation, Option<Country>, Option<OrganisationType>)) -> Self {
        let country = if let Some(c) = country {
            Some(Country {
                id: c.id,
                name: c.name,
                geo_json: c.geo_json,
                flag: c.flag,
                capital_city: c.capital_city,
                description: c.description,
            })
        } else {
            None
        };
        
        let org_type = if let Some(ot) = org_type {
            Some(OrganisationType {
                id: ot.id,
                org_type: ot.org_type,
                color: ot.color,
            })
        } else {
            None
        };
        
        Self {
            id: org.id,
            name: org.name,
            tel: org.tel,
            email: org.email,
            address: org.address,
            description: org.description,
            location_country: country,
            organisation_type: org_type,
            created_at: org.created_at,
            updated_at: org.updated_at,
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
        let response_content = OrganisationContent::from(
            (item, None, None)
        );
        
        HttpResponse::Ok().json(response_content)
    }
}
