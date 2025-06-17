use std::cmp::min;
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use super::usecases::FetchAllCountriesUsecaseInput;
use actix_web::{
    HttpResponse,
    web::{Data, Query},
};
use serde::Deserialize;

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
