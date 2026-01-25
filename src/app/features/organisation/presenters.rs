use super::entities::Organisation;
use actix_web::{HttpResponse, http::StatusCode};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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
    pub founder_country_id: Option<Uuid>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<Organisation> for OrganisationContent {
    fn from(org: Organisation) -> Self {
        let longitude: Option<f64> = match org.longitude {
            Some(dec_val) => {
                let str = dec_val.to_string();
                let value: f64 = f64::from_str(&str).unwrap_or_default();
                Some(value)
            }
            _ => None,
        };
        let latitude: Option<f64> = match org.latitude {
            Some(dec_val) => {
                let str = dec_val.to_string();
                let value: f64 = f64::from_str(&str).unwrap_or_default();
                Some(value)
            }
            _ => None,
        };

        Self {
            id: org.id,
            name: org.name,
            tel: org.tel,
            email: org.email,
            address: org.address,
            description: org.description,
            location_country_id: org.location_country_id,
            organisation_type_id: org.organisation_type_id,
            founder_country_id: org.founder_country_id,
            latitude: latitude,
            longitude: longitude,
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
        let response_items: Vec<OrganisationContent> =
            items.into_iter().map(OrganisationContent::from).collect();
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
        HttpResponse::<Organisation>::with_body(StatusCode::OK, item)
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

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    fn create_test_organisation() -> Organisation {
        Organisation {
            id: Uuid::new_v4(),
            name: "Test Org".to_string(),
            tel: Some("+1234567890".to_string()),
            email: Some("test@example.com".to_string()),
            address: Some("123 Test St".to_string()),
            description: Some("Test Description".to_string()),
            location_country_id: Some(Uuid::new_v4()),
            organisation_type_id: Some(Uuid::new_v4()),
            founder_country_id: Some(Uuid::new_v4()),
            latitude: Some(BigDecimal::from_str("40.7128").unwrap()),
            longitude: Some(BigDecimal::from_str("-74.0060").unwrap()),
            created_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn test_organisation_content_from_organisation() {
        let org = create_test_organisation();
        let org_id = org.id;
        let org_name = org.name.clone();

        let content = OrganisationContent::from(org);

        assert_eq!(content.id, org_id);
        assert_eq!(content.name, org_name);
        assert_eq!(content.tel, Some("+1234567890".to_string()));
        assert_eq!(content.email, Some("test@example.com".to_string()));
        assert_eq!(content.address, Some("123 Test St".to_string()));
        assert!(content.latitude.is_some());
        assert!(content.longitude.is_some());
    }

    #[test]
    fn test_organisation_content_from_organisation_with_null_coords() {
        let mut org = create_test_organisation();
        org.latitude = None;
        org.longitude = None;

        let content = OrganisationContent::from(org);

        assert!(content.latitude.is_none());
        assert!(content.longitude.is_none());
    }

    #[test]
    fn test_multiple_organisations_response_from_vec() {
        let org1 = create_test_organisation();
        let org2 = create_test_organisation();
        let orgs = vec![org1, org2];

        let response = MultipleOrganisationsResponse::from(orgs);

        assert_eq!(response.total, 2);
        assert_eq!(response.items.len(), 2);
    }

    #[test]
    fn test_multiple_organisations_response_empty() {
        let orgs: Vec<Organisation> = vec![];

        let response = MultipleOrganisationsResponse::from(orgs);

        assert_eq!(response.total, 0);
        assert_eq!(response.items.len(), 0);
    }

    #[test]
    fn test_organisation_presenter_new() {
        let presenter = OrganisationPresenterImpl::new();
        assert!(std::mem::size_of_val(&presenter) == 0);
    }

    #[test]
    fn test_organisation_presenter_to_http_res() {
        let presenter = OrganisationPresenterImpl::new();
        let response = presenter.to_http_res();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_organisation_presenter_to_single_json() {
        let presenter = OrganisationPresenterImpl::new();
        let org = create_test_organisation();

        let response = presenter.to_single_json(org);

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_organisation_presenter_to_multi_json() {
        let presenter = OrganisationPresenterImpl::new();
        let org1 = create_test_organisation();
        let org2 = create_test_organisation();
        let orgs = vec![org1, org2];

        let response = presenter.to_multi_json(orgs);

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_organisation_presenter_clone() {
        let presenter = OrganisationPresenterImpl::new();
        let _cloned = presenter.clone();

        // Should not panic and should be cloneable
        assert!(true);
    }
}
