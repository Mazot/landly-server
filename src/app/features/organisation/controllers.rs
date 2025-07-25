use super::{
    requests::{CreateOrganisationRequest, OrganisationsListQueryRequest, UpdateOrganisationRequest},
    usecases::{CreateOrganisationUsecaseInput, FetchOrganisationsUsecaseInput, UpdateOrganisationUsecaseInput},
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{web::{Data, Json, Path, Query}, HttpRequest, HttpResponse};
use bigdecimal::BigDecimal;
use uuid::Uuid;
use std::cmp::min;

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
    query: Query<OrganisationsListQueryRequest>
) -> Result<HttpResponse, AppError> {
    let offset = min(query.offset.unwrap_or(0), 150);
    let limit = query.limit.unwrap_or(20);

    state
        .di_container
        .organisation_usecase
        .fetch_organisations(
            FetchOrganisationsUsecaseInput {
                name: query.name.clone(),
                tel: query.tel.clone(),
                email: query.email.clone(),
                address: query.address.clone(),
                location_country_id: query.location_country_id,
                organisation_type_id: query.organisation_type_id,
                limit,
                offset,
            }
        )
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
    id: Path<Uuid>
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
    _req: HttpRequest,
    form: Json<UpdateOrganisationRequest>,
    id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .update_organisation(
            id.into_inner(),
            UpdateOrganisationUsecaseInput {
                name: form.name.clone(),
                tel: form.tel.clone(),
                email: form.email.clone(),
                address: form.address.clone(),
                description: form.description.clone(),
                location_country_id: form.location_country_id,
                organisation_type_id: form.organisation_type_id,
                latitude: form.latitude.map(BigDecimal::try_from)
                    .map(|v| v.expect("Invalid latitude value")),
                longitude: form.longitude.map(BigDecimal::try_from)
                    .map(|v| v.expect("Invalid longitude value")),
            }
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
    _req: HttpRequest,
    id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .delete_organisation(id.into_inner())
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
    _req: HttpRequest,
    form: Json<CreateOrganisationRequest>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .create_organisation(
            CreateOrganisationUsecaseInput{
                name: form.name.clone(),
                tel: form.tel.clone(),
                email: form.email.clone(),
                address: form.address.clone(),
                description: form.description.clone(),
                location_country_id: form.location_country_id,
                organisation_type_id: form.organisation_type_id,
                latitude: form.latitude.map(BigDecimal::try_from)
                    .map(|v| v.expect("Invalid latitude value")),
                longitude: form.longitude.map(BigDecimal::try_from)
                    .map(|v| v.expect("Invalid longitude value")),
            }
        )
}
