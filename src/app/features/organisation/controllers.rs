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
use std::cmp::min;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

/// Converts an optional f64 coordinate into a BigDecimal, rejecting
/// non-finite values with a 422 instead of panicking.
fn parse_coordinate(value: Option<f64>, field: &str) -> Result<Option<BigDecimal>, AppError> {
    value
        .map(|v| {
            BigDecimal::try_from(v).map_err(|_| {
                AppError::UnprocessableEntity(json!({
                    "error": format!("Invalid {} value", field)
                }))
            })
        })
        .transpose()
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
    let offset = min(query.offset.unwrap_or(0), 150);
    let limit = query.limit.unwrap_or(20);

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
            latitude: parse_coordinate(form.latitude, "latitude")?,
            longitude: parse_coordinate(form.longitude, "longitude")?,
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
            latitude: parse_coordinate(form.latitude, "latitude")?,
            longitude: parse_coordinate(form.longitude, "longitude")?,
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
            limit: min(query.limit.unwrap_or(50), 200),
            offset: query.offset.unwrap_or(0),
        })
}

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
