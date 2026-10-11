use crate::data::models::{Country, OrganisationType};
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub trait CommonPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_country_json(&self, item: Country) -> HttpResponse;
    fn to_country_detail_json(&self, item: Country, by_type: Vec<(String, i64)>) -> HttpResponse;
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

    fn to_country_detail_json(&self, item: Country, by_type: Vec<(String, i64)>) -> HttpResponse {
        let response_content = CountryDetailContent::from((item, by_type));

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
    /// Country ID.
    pub id: Uuid,
    /// Country name in English, e.g. "Germany".
    pub name: String,
    /// Country border as a GeoJSON geometry (`Polygon` / `MultiPolygon`) for map rendering.
    pub geo_json: Option<Value>,
    /// Flag emoji, e.g. "🇩🇪".
    pub flag: Option<String>,
    /// Capital city name.
    pub capital_city: Option<String>,
    /// Free-text description shown on the country page.
    pub description: Option<String>,
    /// Local currency, e.g. "EUR" (free text, not validated).
    pub currency: Option<String>,
    /// International dialling prefix, e.g. "+49".
    pub phone_code: Option<String>,
    /// Main cities for the country page, as stored JSON (typically an array of city names).
    pub top_cities: Option<Value>,
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
            currency: val.currency,
            phone_code: val.phone_code,
            top_cities: val.top_cities,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryPlacesByType {
    /// Organisation type slug; "other" for types without a slug.
    pub slug: String,
    /// Number of live organisations of this type in the country.
    pub count: i64,
}

/// Country page payload with live-organisation breakdown by org type
/// (design: country-full.jsx).
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryDetailContent {
    /// The country itself (flattened into this object).
    #[serde(flatten)]
    pub country: CountryContent,
    /// Number of live organisations located in the country.
    pub total_places: i64,
    /// Live organisations per organisation type.
    pub places_by_type: Vec<CountryPlacesByType>,
}

impl From<(Country, Vec<(String, i64)>)> for CountryDetailContent {
    fn from((country, by_type): (Country, Vec<(String, i64)>)) -> Self {
        let mut total_places = 0;
        let places_by_type = by_type
            .into_iter()
            .map(|(slug, count)| {
                total_places += count;
                CountryPlacesByType { slug, count }
            })
            .collect();

        Self {
            country: CountryContent::from(country),
            total_places,
            places_by_type,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationTypeContent {
    /// Organisation type ID.
    pub id: Uuid,
    /// Type key, e.g. "embassy".
    pub r#type: String,
    /// Map pin / badge colour as a hex string, e.g. "#4A6D8C".
    pub color: Option<String>,
    /// Human-readable name, e.g. "Embassy".
    pub title: Option<String>,
    /// Stable machine key used by filters and stats (`embassy`, `business`, `helper`, `community`, `volunteer`); unique when set.
    pub slug: Option<String>,
}
impl From<OrganisationType> for OrganisationTypeContent {
    fn from(val: OrganisationType) -> Self {
        Self {
            id: val.id,
            r#type: val.org_type,
            color: val.color,
            title: val.title,
            slug: val.slug,
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
            currency: Some("EUR".to_string()),
            phone_code: Some("+49".to_string()),
            top_cities: Some(serde_json::json!(["Berlin", "Munich"])),
        }
    }

    fn create_test_org_type() -> OrganisationType {
        OrganisationType {
            id: Uuid::new_v4(),
            org_type: "test_type".to_string(),
            color: Some("#FF0000".to_string()),
            title: Some("Test Type".to_string()),
            slug: Some("test_type".to_string()),
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
