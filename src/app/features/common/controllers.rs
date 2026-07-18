use super::usecases::FetchAllCountriesUsecaseInput;
use crate::app::{
    drivers::middlewares::state::AppState,
    features::common::usecases::CreateOrganisationTypeUsecaseInput,
};
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use serde::Deserialize;
use serde_json::json;
use std::cmp::min;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

#[utoipa::path(
    get,
    path = "/common/countries",
    context_path = "/api",
    responses(
        (status = 200, description = "Countries list response", body = Vec<super::presenters::CountryContent>),
    ),
    params(CountriesListQueryParams),
    tag = "Common"
)]
pub async fn fetch_all_countries(
    state: Data<AppState>,
    params: Query<CountriesListQueryParams>,
) -> Result<HttpResponse, AppError> {
    let offset = min(params.offset.unwrap_or(0), 150);
    let limit = params.limit.unwrap_or(20);
    let name = params.name.clone();

    state
        .di_container
        .common_usecase
        .fetch_all_countries(FetchAllCountriesUsecaseInput {
            limit,
            offset,
            name,
        })
}

#[utoipa::path(
    get,
    path = "/common/countries/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country ID")
    ),
    responses(
        (status = 200, description = "Country with live-organisation breakdown by type", body = super::presenters::CountryDetailContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn fetch_country_detail(
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .common_usecase
        .fetch_country_detail(id.into_inner())
}

#[utoipa::path(
    get,
    path = "/common/org_types",
    context_path = "/api",
    responses(
        (status = 200, description = "Organisation types list response", body = Vec<super::presenters::OrganisationTypeContent>),
    ),
    tag = "Common"
)]
pub async fn fetch_all_organisation_types(state: Data<AppState>) -> Result<HttpResponse, AppError> {
    state.di_container.common_usecase.fetch_organisation_types()
}

#[utoipa::path(
    post,
    path = "/common/org_types",
    context_path = "/api",
    request_body = CreateOrganisationTypeRequest,
    responses(
        (status = 200, description = "Organisation type created successfully", body = super::presenters::OrganisationTypeContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (admin only)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn create_organisation_type(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateOrganisationTypeRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.common_usecase.create_organisation_type(
        caller,
        CreateOrganisationTypeUsecaseInput {
            org_type: form.org_type.to_owned(),
            color: form.color.to_owned(),
            title: form.title.to_owned(),
            slug: form.slug.to_owned(),
        },
    )
}

#[derive(Deserialize, ToSchema)]
pub struct CreateOrganisationTypeRequest {
    pub org_type: String,
    pub color: String,
    pub title: String,
    pub slug: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct CountriesListQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub name: Option<String>,
}
