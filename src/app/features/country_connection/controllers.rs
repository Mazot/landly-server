use super::{
    requests::{CreateCountryConnectionRequest, CountryConnectionsListQueryParams, UpdateCountryConnectionRequest},
    usecases::{CreateCountryConnectionUsecaseInput, FetchCountryConnectionsUsecaseInput, UpdateCountryConnectionUsecaseInput},
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{web::{Data, Json, Path, Query}, HttpRequest, HttpResponse};
use uuid::Uuid;
use std::cmp::min;

#[utoipa::path(
    get,
    path = "/country-connection/list",
    context_path = "/api",
    params(CountryConnectionsListQueryParams),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::MultipleCountryConnectionsResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn list(
    state: Data<AppState>,
    query: Query<CountryConnectionsListQueryParams>
) -> Result<HttpResponse, AppError> {
    let offset = min(query.offset.unwrap_or(0), 150);
    let limit = query.limit.unwrap_or(20);

    state
        .di_container
        .country_connection_usecase
        .fetch_country_connections(
            FetchCountryConnectionsUsecaseInput {
                embassy_org_id: query.embassy_org_id,
                consulate_org_id: query.consulate_org_id,
                location_country_id: query.location_country_id,
                limit,
                offset,
            }
        )
}

#[utoipa::path(
    get,
    path = "/country-connection/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to fetch")
    ),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn fetch(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .country_connection_usecase
        .fetch_country_connection(id.into_inner())
}

#[utoipa::path(
    post,
    path = "/country-connection/create",
    context_path = "/api",
    request_body = CreateCountryConnectionRequest,
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn create(
    state: Data<AppState>,
    _req: HttpRequest,
    form: Json<CreateCountryConnectionRequest>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .country_connection_usecase
        .create_country_connection(
            CreateCountryConnectionUsecaseInput {
                embassy_org_id: form.embassy_org_id,
                consulate_org_id: form.consulate_org_id,
                common_info: form.common_info.clone(),
                location_country_id: form.location_country_id,
            }
        )
}

#[utoipa::path(
    put,
    path = "/country-connection/update/{id}",
    context_path = "/api",
    request_body = UpdateCountryConnectionRequest,
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to update")
    ),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn update(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>,
    form: Json<UpdateCountryConnectionRequest>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .country_connection_usecase
        .update_country_connection(
            id.into_inner(),
            UpdateCountryConnectionUsecaseInput {
                embassy_org_id: form.embassy_org_id,
                consulate_org_id: form.consulate_org_id,
                location_country_id: form.location_country_id,
                common_info: form.common_info.clone(),
            }
        )
}

#[utoipa::path(
    delete,
    path = "/country-connection/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to delete")
    ),
    responses(
        (status = 200, description = "Country connection deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn delete(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .country_connection_usecase
        .delete_country_connection(id.into_inner())
}
