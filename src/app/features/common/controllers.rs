use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use super::usecases::FetchAllCountriesUsecaseInput;
use actix_web::{
    HttpResponse,
    web::{Data, Query},
};
use std::cmp::min;
use serde::Deserialize;

#[utoipa::path(
    get,
    path = "/common/countries",
    context_path = "/api",
    responses(
        (status = 200, description = "Countries list response", body = Vec<super::presenters::CountryContent>),
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Optional limit for the number of countries to fetch, default is 20"),
        ("offset" = Option<i64>, Query, description = "Optional offset for pagination, default is 0")
    ),
    tag = "Common"
)]
pub async fn fetch_all_countries(
    state: Data<AppState>,
    params: Query<CountriesListQueryParams>,
) -> Result<HttpResponse, AppError> {
    let offset = min(params.offset.unwrap_or(0), 150);
    let limit = params.limit.unwrap_or(20);

    state
        .di_container
        .common_usecase
        .fetch_all_countries(
            FetchAllCountriesUsecaseInput {
                limit,
                offset,
            }
        )
}

#[derive(Deserialize)]
pub struct CountriesListQueryParams {
    limit: Option<i64>,
    offset: Option<i64>,
}
