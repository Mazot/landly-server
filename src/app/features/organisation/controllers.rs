use super::{
    requests::{
        CreateOrganisationRequest, OrganisationsListQueryRequest, SearchOrganisationsQueryRequest,
        UpdateOrganisationRequest,
    },
    usecases::{
        CreateOrganisationUsecaseInput, FetchOrganisationsUsecaseInput,
        SearchOrganisationsUsecaseInput, UpdateOrganisationUsecaseInput,
    },
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use bigdecimal::BigDecimal;
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

const LATITUDE_RANGE: std::ops::RangeInclusive<f64> = -90.0..=90.0;
const LONGITUDE_RANGE: std::ops::RangeInclusive<f64> = -180.0..=180.0;

/// Converts an optional f64 coordinate into a BigDecimal, rejecting
/// non-finite and out-of-range values with a 422 instead of panicking
/// (or silently storing garbage like lat=999).
fn parse_coordinate(
    value: Option<f64>,
    field: &str,
    range: std::ops::RangeInclusive<f64>,
) -> Result<Option<BigDecimal>, AppError> {
    value
        .map(|v| {
            if !v.is_finite() || !range.contains(&v) {
                return Err(AppError::UnprocessableEntity(json!({
                    "error": format!(
                        "Invalid {} value: must be within [{}, {}]",
                        field,
                        range.start(),
                        range.end()
                    )
                })));
            }

            BigDecimal::try_from(v).map_err(|_| {
                AppError::UnprocessableEntity(json!({
                    "error": format!("Invalid {} value", field)
                }))
            })
        })
        .transpose()
}

fn parse_latitude(value: Option<f64>) -> Result<Option<BigDecimal>, AppError> {
    parse_coordinate(value, "latitude", LATITUDE_RANGE)
}

fn parse_longitude(value: Option<f64>) -> Result<Option<BigDecimal>, AppError> {
    parse_coordinate(value, "longitude", LONGITUDE_RANGE)
}

/// Validates a search origin/bbox coordinate pair without converting it.
fn ensure_coordinate_ranges(pairs: &[(&str, f64)]) -> Result<(), AppError> {
    for (field, value) in pairs {
        let range = if field.contains("lat") {
            LATITUDE_RANGE
        } else {
            LONGITUDE_RANGE
        };

        if !value.is_finite() || !range.contains(value) {
            return Err(AppError::UnprocessableEntity(json!({
                "error": format!(
                    "Invalid {} value: must be within [{}, {}]",
                    field,
                    range.start(),
                    range.end()
                )
            })));
        }
    }

    Ok(())
}

/// Splits a comma-separated query value into a list, dropping empty chunks.
fn parse_csv(value: &Option<String>) -> Option<Vec<String>> {
    let items: Vec<String> = value
        .as_deref()?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if items.is_empty() { None } else { Some(items) }
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// results are now filtered to status='live'.
#[utoipa::path(
    get,
    path = "/organisation/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Organisations list response", body = super::presenters::MultipleOrganisationsResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(OrganisationsListQueryRequest),
    tag = "Organisation"
)]
pub async fn list_organisations(
    state: Data<AppState>,
    query: Query<OrganisationsListQueryRequest>,
) -> Result<HttpResponse, AppError> {
    // Clamp to sane bounds: negative values reach Postgres as
    // `OFFSET -n` / `LIMIT -n` and blow up with a 500.
    let offset = query.offset.unwrap_or(0).clamp(0, 150);
    let limit = query.limit.unwrap_or(20).clamp(0, 100);

    state
        .di_container
        .organisation_usecase
        .fetch_organisations(FetchOrganisationsUsecaseInput {
            name: query.name.clone(),
            tel: query.tel.clone(),
            email: query.email.clone(),
            address: query.address.clone(),
            location_country_id: query.location_country_id,
            organisation_type_id: query.organisation_type_id,
            founder_country_id: query.founder_country_id,
            limit,
            offset,
        })
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// response carries the v2 fields incl. computed openNow.
#[utoipa::path(
    get,
    path = "/organisation/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID to fetch")
    ),
    responses(
        (status = 200, description = "Organisation fetched successfully", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn fetch_organisation(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .fetch_organisation(id.into_inner())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// ownership/RBAC check, v2 fields, panic-free coordinate parsing.
#[utoipa::path(
    put,
    path = "/organisation/update/{id}",
    context_path = "/api",
    request_body = UpdateOrganisationRequest,
    params(
        ("id" = Uuid, Path, description = "Organisation ID to update")
    ),
    responses(
        (status = 200, description = "Organisation updated successfully", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn update_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    form: Json<UpdateOrganisationRequest>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.organisation_usecase.update_organisation(
        id.into_inner(),
        caller,
        UpdateOrganisationUsecaseInput {
            name: form.name.clone(),
            tel: form.tel.clone(),
            email: form.email.clone(),
            address: form.address.clone(),
            description: form.description.clone(),
            location_country_id: form.location_country_id,
            organisation_type_id: form.organisation_type_id,
            founder_country_id: form.founder_country_id,
            latitude: parse_latitude(form.latitude)?,
            longitude: parse_longitude(form.longitude)?,
            city: form.city.clone(),
            website: form.website.clone(),
            telegram: form.telegram.clone(),
            whatsapp: form.whatsapp.clone(),
            services: form.services.clone(),
            languages: form.languages.clone(),
            opening_hours: form.opening_hours.clone(),
            timezone: form.timezone.clone(),
            cost: form.cost.clone(),
        },
    )
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// ownership/RBAC check.
#[utoipa::path(
    delete,
    path = "/organisation/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID to delete")
    ),
    responses(
        (status = 200, description = "Organisation deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn delete_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .organisation_usecase
        .delete_organisation(id.into_inner(), caller)
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// created_by from JWT, v2 fields, submissions start as status='pending'.
#[utoipa::path(
    post,
    path = "/organisation/create",
    context_path = "/api",
    request_body = CreateOrganisationRequest,
    responses(
        (status = 200, description = "Organisation created successfully", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn create_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    form: Json<CreateOrganisationRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .organisation_usecase
        .create_organisation(CreateOrganisationUsecaseInput {
            name: form.name.clone(),
            tel: form.tel.clone(),
            email: form.email.clone(),
            address: form.address.clone(),
            description: form.description.clone(),
            location_country_id: form.location_country_id,
            organisation_type_id: form.organisation_type_id,
            founder_country_id: form.founder_country_id,
            latitude: parse_latitude(form.latitude)?,
            longitude: parse_longitude(form.longitude)?,
            created_by: caller,
            added_by: form.added_by.clone(),
            city: form.city.clone(),
            website: form.website.clone(),
            telegram: form.telegram.clone(),
            whatsapp: form.whatsapp.clone(),
            services: form.services.clone().unwrap_or_default(),
            languages: form.languages.clone().unwrap_or_default(),
            opening_hours: form.opening_hours.clone(),
            timezone: form.timezone.clone(),
            cost: form.cost.clone(),
            google_place_id: form.google_place_id.clone(),
        })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/organisation/search",
    context_path = "/api",
    responses(
        (status = 200, description = "Geo search results", body = super::presenters::MultipleOrganisationsResponse),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(SearchOrganisationsQueryRequest),
    tag = "Organisation"
)]
pub async fn search_organisations(
    state: Data<AppState>,
    query: Query<SearchOrganisationsQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let bbox = match (query.min_lat, query.min_lng, query.max_lat, query.max_lng) {
        (Some(min_lat), Some(min_lng), Some(max_lat), Some(max_lng)) => {
            Some((min_lat, min_lng, max_lat, max_lng))
        }
        (None, None, None, None) => None,
        _ => {
            return Err(AppError::UnprocessableEntity(json!({
                "error": "bbox requires all of min_lat, min_lng, max_lat, max_lng"
            })));
        }
    };

    let origin = match (query.lat, query.lng) {
        (Some(lat), Some(lng)) => Some((lat, lng)),
        (None, None) => None,
        _ => {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "origin requires both lat and lng" }),
            ));
        }
    };

    if bbox.is_none() && origin.is_none() {
        return Err(AppError::UnprocessableEntity(json!({
            "error": "Provide a bbox (min_lat..max_lng) or an origin (lat, lng)"
        })));
    }

    if let Some((min_lat, min_lng, max_lat, max_lng)) = bbox {
        ensure_coordinate_ranges(&[
            ("min_lat", min_lat),
            ("min_lng", min_lng),
            ("max_lat", max_lat),
            ("max_lng", max_lng),
        ])?;
    }
    if let Some((lat, lng)) = origin {
        ensure_coordinate_ranges(&[("lat", lat), ("lng", lng)])?;
    }

    state
        .di_container
        .organisation_usecase
        .search_organisations(SearchOrganisationsUsecaseInput {
            bbox,
            origin,
            radius_km: query.radius_km,
            type_slugs: parse_csv(&query.types),
            open_now: query.open_now.unwrap_or(false),
            languages: parse_csv(&query.languages),
            verified_only: query.verified.unwrap_or(false),
            min_rating: query.min_rating,
            added_by: query.added_by.clone(),
            cost: query.cost.clone(),
            sort: query.sort.clone(),
            limit: query.limit.unwrap_or(50).clamp(0, 200),
            offset: query.offset.unwrap_or(0).max(0),
        })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    post,
    path = "/organisation/visit/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID that was visited")
    ),
    responses(
        (status = 200, description = "Visit counted", body = super::presenters::OrganisationVisitsContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn visit_organisation(
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .visit_organisation(id.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coordinate_valid() {
        assert!(parse_latitude(Some(52.52)).unwrap().is_some());
        assert!(parse_longitude(Some(-179.9)).unwrap().is_some());
        assert!(parse_latitude(Some(-90.0)).unwrap().is_some());
        assert!(parse_longitude(Some(180.0)).unwrap().is_some());
    }

    #[test]
    fn test_parse_coordinate_none() {
        assert!(parse_latitude(None).unwrap().is_none());
        assert!(parse_longitude(None).unwrap().is_none());
    }

    #[test]
    fn test_parse_coordinate_rejects_non_finite() {
        // Previously these panicked via .expect(); now they must be a 422.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            match parse_latitude(Some(bad)) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
            }
        }
    }

    #[test]
    fn test_parse_coordinate_rejects_out_of_range() {
        assert!(parse_latitude(Some(90.01)).is_err());
        assert!(parse_latitude(Some(-999.0)).is_err());
        assert!(parse_longitude(Some(180.5)).is_err());
        assert!(parse_longitude(Some(-181.0)).is_err());
    }

    #[test]
    fn test_ensure_coordinate_ranges() {
        assert!(ensure_coordinate_ranges(&[("lat", 52.5), ("lng", 13.4)]).is_ok());
        assert!(ensure_coordinate_ranges(&[("min_lat", 999.0)]).is_err());
        assert!(ensure_coordinate_ranges(&[("max_lng", -200.0)]).is_err());
    }

    #[test]
    fn test_parse_csv_basic() {
        let input = Some("embassy,community".to_string());
        assert_eq!(
            parse_csv(&input),
            Some(vec!["embassy".to_string(), "community".to_string()])
        );
    }

    #[test]
    fn test_parse_csv_trims_and_drops_empty_chunks() {
        let input = Some(" embassy , ,community, ".to_string());
        assert_eq!(
            parse_csv(&input),
            Some(vec!["embassy".to_string(), "community".to_string()])
        );
    }

    #[test]
    fn test_parse_csv_empty_inputs() {
        assert_eq!(parse_csv(&None), None);
        assert_eq!(parse_csv(&Some("".to_string())), None);
        assert_eq!(parse_csv(&Some(", ,".to_string())), None);
    }
}
