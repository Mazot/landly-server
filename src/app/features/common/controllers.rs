use super::usecases::FetchAllCountriesUsecaseInput;
use crate::app::{
    drivers::middlewares::state::AppState,
    features::common::usecases::CreateOrganisationTypeUsecaseInput,
};
use crate::error::AppError;
use actix_web::{
    HttpResponse,
    web::{Data, Json, Query},
};
use serde::Deserialize;
use std::cmp::min;
use utoipa::{IntoParams, ToSchema};

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
        .fetch_all_countries(FetchAllCountriesUsecaseInput { limit, offset, name })
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
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn create_organisation_type(
    state: Data<AppState>,
    form: Json<CreateOrganisationTypeRequest>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .common_usecase
        .create_organisation_type(CreateOrganisationTypeUsecaseInput {
            org_type: form.org_type.to_owned(),
            color: form.color.to_owned(),
            title: form.title.to_owned(),
        })
}

#[derive(Deserialize, ToSchema)]
pub struct CreateOrganisationTypeRequest {
    pub org_type: String,
    pub color: String,
    pub title: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct CountriesListQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub name: Option<String>,
}
