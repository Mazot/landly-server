use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::data::models::{Country, OrganisationType};

pub trait CommonPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_country_json(&self, item: Country) -> HttpResponse;
    fn to_multi_country_json(&self, item: Vec<Country>) -> HttpResponse;
    fn to_multi_organization_type_json(&self, item: Vec<OrganisationType>) -> HttpResponse;
}

#[derive(Clone)]
pub struct CommonPresenterImpl {}
impl CommonPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl CommonPresenter for CommonPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_country_json(&self, item: Country) -> HttpResponse {
        let response_content = CountryContent::from(item);

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_country_json(&self, item: Vec<Country>) -> HttpResponse {
        let response_content: Vec<CountryContent> = item.iter()
            .map(|country| CountryContent::from(country.to_owned()))
            .collect();

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_organization_type_json(&self, item: Vec<OrganisationType>) -> HttpResponse {
        let response_content: Vec<OrganisationTypeContent> = item.iter()
            .map(|org_type| OrganisationTypeContent::from(org_type.to_owned()))
            .collect();

        HttpResponse::Ok().json(response_content)
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryContent {
    pub id: Uuid,
    pub name: String,
    pub geo_json: Option<Value>,
    pub flag: Option<String>,
    pub capital_city: Option<String>,
    pub description: Option<String>,
}
impl From<Country> for CountryContent {
    fn from(val: Country) -> Self {
        Self {
            id: val.id,
            name: val.name.to_owned(),
            geo_json: val.geo_json,
            flag: val.flag,
            capital_city: val.capital_city,
            description: val.description
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationTypeContent {
    pub id: Uuid,
    pub r#type: String,
    pub color: Option<String>,
}
impl From<OrganisationType> for OrganisationTypeContent {
    fn from(val: OrganisationType) -> Self {
        Self {
            id: val.id,
            r#type: val.org_type,
            color: val.color,
        }
    }
}
