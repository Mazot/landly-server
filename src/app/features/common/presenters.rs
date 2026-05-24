use crate::data::models::{Country, OrganisationType};
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub trait CommonPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_country_json(&self, item: Country) -> HttpResponse;
    fn to_single_organization_type_json(&self, item: OrganisationType) -> HttpResponse;
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
        let response_content: Vec<CountryContent> = item
            .iter()
            .map(|country| CountryContent::from(country.to_owned()))
            .collect();

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_organization_type_json(&self, item: Vec<OrganisationType>) -> HttpResponse {
        let response_content: Vec<OrganisationTypeContent> = item
            .iter()
            .map(|org_type| OrganisationTypeContent::from(org_type.to_owned()))
            .collect();

        HttpResponse::Ok().json(response_content)
    }

    fn to_single_organization_type_json(&self, item: OrganisationType) -> HttpResponse {
        let response_content = OrganisationTypeContent::from(item);

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
            description: val.description,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationTypeContent {
    pub id: Uuid,
    pub r#type: String,
    pub color: Option<String>,
    pub title: Option<String>,
}
impl From<OrganisationType> for OrganisationTypeContent {
    fn from(val: OrganisationType) -> Self {
        Self {
            id: val.id,
            r#type: val.org_type,
            color: val.color,
            title: val.title,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_country() -> Country {
        Country {
            id: Uuid::new_v4(),
            name: "Test Country".to_string(),
            geo_json: Some(serde_json::json!({"type": "Feature"})),
            flag: Some("🏴".to_string()),
            capital_city: Some("Test Capital".to_string()),
            description: Some("Test Description".to_string()),
        }
    }

    fn create_test_org_type() -> OrganisationType {
        OrganisationType {
            id: Uuid::new_v4(),
            org_type: "test_type".to_string(),
            color: Some("#FF0000".to_string()),
            title: Some("Test Type".to_string()),
        }
    }

    #[test]
    fn test_country_content_from_country() {
        let country = create_test_country();
        let country_id = country.id;
        let country_name = country.name.clone();

        let content = CountryContent::from(country);

        assert_eq!(content.id, country_id);
        assert_eq!(content.name, country_name);
        assert_eq!(content.flag, Some("🏴".to_string()));
        assert_eq!(content.capital_city, Some("Test Capital".to_string()));
        assert!(content.geo_json.is_some());
    }

    #[test]
    fn test_organisation_type_content_from_org_type() {
        let org_type = create_test_org_type();
        let org_type_id = org_type.id;

        let content = OrganisationTypeContent::from(org_type);

        assert_eq!(content.id, org_type_id);
        assert_eq!(content.r#type, "test_type");
        assert_eq!(content.color, Some("#FF0000".to_string()));
        assert_eq!(content.title, Some("Test Type".to_string()));
    }

    #[test]
    fn test_common_presenter_new() {
        let presenter = CommonPresenterImpl::new();
        assert!(std::mem::size_of_val(&presenter) == 0);
    }

    #[test]
    fn test_common_presenter_to_http_res() {
        let presenter = CommonPresenterImpl::new();
        let response = presenter.to_http_res();

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_common_presenter_to_single_country_json() {
        let presenter = CommonPresenterImpl::new();
        let country = create_test_country();

        let response = presenter.to_single_country_json(country);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_common_presenter_to_multi_country_json() {
        let presenter = CommonPresenterImpl::new();
        let country1 = create_test_country();
        let country2 = create_test_country();
        let countries = vec![country1, country2];

        let response = presenter.to_multi_country_json(countries);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_common_presenter_to_single_organization_type_json() {
        let presenter = CommonPresenterImpl::new();
        let org_type = create_test_org_type();

        let response = presenter.to_single_organization_type_json(org_type);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_common_presenter_to_multi_organization_type_json() {
        let presenter = CommonPresenterImpl::new();
        let org_type1 = create_test_org_type();
        let org_type2 = create_test_org_type();
        let org_types = vec![org_type1, org_type2];

        let response = presenter.to_multi_organization_type_json(org_types);

        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_common_presenter_clone() {
        let presenter = CommonPresenterImpl::new();
        let _cloned = presenter.clone();
    }
}
