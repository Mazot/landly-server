use super::entities::{CommunitySignals, Organisation};
use actix_web::{HttpResponse, http::StatusCode};
use chrono::{Datelike, NaiveDateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait OrganisationPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation>;
    fn to_single_json(&self, item: Organisation) -> HttpResponse;
    fn to_single_with_community_json(
        &self,
        item: Organisation,
        signals: CommunitySignals,
    ) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<Organisation>) -> HttpResponse;
    fn to_search_json(&self, items: Vec<(Organisation, Option<f64>)>) -> HttpResponse;
    fn to_visits_json(&self, visits: i64) -> HttpResponse;
}

/// Computes whether a place is open right now from its structured hours and
/// IANA timezone. `opening_hours` format: {"mon": [["09:00","13:00"], ...], ...}
/// Returns None when hours or timezone are missing/invalid — the client shows
/// "hours unknown" in that case.
pub fn compute_open_now(
    opening_hours: &Option<serde_json::Value>,
    timezone: &Option<String>,
) -> Option<bool> {
    let hours = opening_hours.as_ref()?.as_object()?;
    let tz: chrono_tz::Tz = timezone.as_deref()?.parse().ok()?;
    let now = chrono::Utc::now().with_timezone(&tz);

    let day_key = match now.weekday() {
        chrono::Weekday::Mon => "mon",
        chrono::Weekday::Tue => "tue",
        chrono::Weekday::Wed => "wed",
        chrono::Weekday::Thu => "thu",
        chrono::Weekday::Fri => "fri",
        chrono::Weekday::Sat => "sat",
        chrono::Weekday::Sun => "sun",
    };

    let intervals = match hours.get(day_key) {
        Some(v) => v.as_array()?,
        None => return Some(false),
    };

    // "HH:MM" strings compare correctly lexicographically.
    let current = now.format("%H:%M").to_string();

    Some(intervals.iter().any(|interval| {
        interval
            .as_array()
            .and_then(|pair| {
                let open = pair.first()?.as_str()?;
                let close = pair.get(1)?.as_str()?;
                Some(open <= current.as_str() && current.as_str() < close)
            })
            .unwrap_or(false)
    }))
}

fn decimal_to_f64(value: Option<bigdecimal::BigDecimal>) -> Option<f64> {
    // ToPrimitive instead of a string round-trip: no silent 0.0 fallback
    // that would place an unparseable coordinate in the Gulf of Guinea.
    value.and_then(|dec_val| bigdecimal::ToPrimitive::to_f64(&dec_val))
}

fn flatten_text_array(values: Vec<Option<String>>) -> Vec<String> {
    values.into_iter().flatten().collect()
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
    pub created_by: Option<Uuid>,
    pub verified: bool,
    pub status: String,
    pub added_by: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Vec<String>,
    pub languages: Vec<String>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    /// Computed from opening_hours + timezone at response time; never stored.
    pub open_now: Option<bool>,
    pub cost: Option<String>,
    pub google_place_id: Option<String>,
    pub google_rating: Option<f64>,
    pub visits_count: i64,
    pub rating_avg: Option<f64>,
    pub reviews_count: i64,
    /// Distance from the search origin in km; present only in /search
    /// responses when an origin point was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f64>,
    /// Community check-in signals; present only on the detail (fetch) payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community: Option<CommunitySignalsContent>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// "Community check-ins" block (design: org-full.jsx).
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommunitySignalsContent {
    pub people_came: i64,
    /// Percentage of check-ins confirming "still active"; None with no data
    pub still_active_pct: Option<f64>,
    pub last_checkin_at: Option<NaiveDateTime>,
    pub tips: Vec<String>,
}

impl From<CommunitySignals> for CommunitySignalsContent {
    fn from(s: CommunitySignals) -> Self {
        Self {
            people_came: s.people_came,
            still_active_pct: s.still_active_pct,
            last_checkin_at: s.last_checkin_at,
            tips: s.tips,
        }
    }
}

impl From<Organisation> for OrganisationContent {
    fn from(org: Organisation) -> Self {
        let open_now = compute_open_now(&org.opening_hours, &org.timezone);

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
            latitude: decimal_to_f64(org.latitude),
            longitude: decimal_to_f64(org.longitude),
            created_by: org.created_by,
            verified: org.verified,
            status: org.status,
            added_by: org.added_by,
            city: org.city,
            website: org.website,
            telegram: org.telegram,
            whatsapp: org.whatsapp,
            services: flatten_text_array(org.services),
            languages: flatten_text_array(org.languages),
            opening_hours: org.opening_hours,
            timezone: org.timezone,
            open_now,
            cost: org.cost,
            google_place_id: org.google_place_id,
            google_rating: org.google_rating,
            visits_count: org.visits_count,
            rating_avg: org.rating_avg,
            reviews_count: org.reviews_count,
            distance_km: None,
            community: None,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

impl From<(Organisation, Option<f64>)> for OrganisationContent {
    fn from((org, distance_km): (Organisation, Option<f64>)) -> Self {
        let mut content = OrganisationContent::from(org);
        content.distance_km = distance_km;

        content
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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationVisitsContent {
    pub visits_count: i64,
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

    fn to_single_with_community_json(
        &self,
        item: Organisation,
        signals: CommunitySignals,
    ) -> HttpResponse {
        let mut response_content = OrganisationContent::from(item);
        response_content.community = Some(CommunitySignalsContent::from(signals));

        HttpResponse::Ok().json(response_content)
    }

    fn to_multi_json(&self, items: Vec<Organisation>) -> HttpResponse {
        let response_content = MultipleOrganisationsResponse::from(items);

        HttpResponse::Ok().json(response_content)
    }

    fn to_search_json(&self, items: Vec<(Organisation, Option<f64>)>) -> HttpResponse {
        let response_items: Vec<OrganisationContent> =
            items.into_iter().map(OrganisationContent::from).collect();
        let total = response_items.len() as i64;

        HttpResponse::Ok().json(MultipleOrganisationsResponse {
            items: response_items,
            total,
        })
    }

    fn to_visits_json(&self, visits: i64) -> HttpResponse {
        HttpResponse::Ok().json(OrganisationVisitsContent {
            visits_count: visits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use serde_json::json;
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
            created_by: Some(Uuid::new_v4()),
            verified: true,
            status: "live".to_string(),
            moderation_note: None,
            added_by: Some("community".to_string()),
            city: Some("New York".to_string()),
            website: Some("https://example.com".to_string()),
            telegram: None,
            whatsapp: None,
            services: vec![Some("Notary".to_string())],
            languages: vec![Some("Russian".to_string()), Some("English".to_string())],
            opening_hours: None,
            timezone: None,
            cost: Some("free".to_string()),
            google_place_id: None,
            google_rating: None,
            visits_count: 7,
            rating_avg: Some(4.5),
            reviews_count: 12,
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
        assert!(content.verified);
        assert_eq!(content.status, "live");
        assert_eq!(content.languages, vec!["Russian", "English"]);
        assert_eq!(content.visits_count, 7);
        assert_eq!(content.rating_avg, Some(4.5));
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
    fn test_organisation_content_with_distance() {
        let org = create_test_organisation();
        let content = OrganisationContent::from((org, Some(12.5)));

        assert_eq!(content.distance_km, Some(12.5));
    }

    #[test]
    fn test_compute_open_now_without_hours() {
        assert_eq!(
            compute_open_now(&None, &Some("Europe/Berlin".to_string())),
            None
        );
        assert_eq!(
            compute_open_now(&Some(json!({"mon": [["09:00", "18:00"]]})), &None),
            None
        );
    }

    #[test]
    fn test_compute_open_now_open_all_day_every_day() {
        let hours = json!({
            "mon": [["00:00", "24:00"]],
            "tue": [["00:00", "24:00"]],
            "wed": [["00:00", "24:00"]],
            "thu": [["00:00", "24:00"]],
            "fri": [["00:00", "24:00"]],
            "sat": [["00:00", "24:00"]],
            "sun": [["00:00", "24:00"]],
        });

        assert_eq!(
            compute_open_now(&Some(hours), &Some("Europe/Berlin".to_string())),
            Some(true)
        );
    }

    #[test]
    fn test_compute_open_now_closed_day_missing() {
        // No entry for any weekday → closed
        let hours = json!({});

        assert_eq!(
            compute_open_now(&Some(hours), &Some("Europe/Berlin".to_string())),
            Some(false)
        );
    }

    #[test]
    fn test_compute_open_now_invalid_timezone() {
        let hours = json!({"mon": [["09:00", "18:00"]]});

        assert_eq!(
            compute_open_now(&Some(hours), &Some("Not/AZone".to_string())),
            None
        );
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
    fn test_organisation_presenter_to_search_json() {
        let presenter = OrganisationPresenterImpl::new();
        let items = vec![
            (create_test_organisation(), Some(1.2)),
            (create_test_organisation(), None),
        ];

        let response = presenter.to_search_json(items);

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_organisation_presenter_to_visits_json() {
        let presenter = OrganisationPresenterImpl::new();
        let response = presenter.to_visits_json(42);

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_organisation_presenter_clone() {
        let presenter = OrganisationPresenterImpl::new();
        let _cloned = presenter.clone(); // Should not panic and should be cloneable
    }
}
