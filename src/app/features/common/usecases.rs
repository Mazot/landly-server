use crate::error::AppError;
use super::{
    presenters::CommonPresenter,
    repositories::{CommonRepository, GetAllCountriesRepositoryInput}
};
use std::sync::Arc;
use actix_web::HttpResponse;

#[derive(Clone)]
pub struct CommonUsecase {
    common_repo: Arc<dyn CommonRepository>,
    common_presenter: Arc<dyn CommonPresenter>,
}

impl CommonUsecase {
    pub fn new(
        common_repo: Arc<dyn CommonRepository>,
        common_presenter: Arc<dyn CommonPresenter>,
    ) -> Self {
        Self {
            common_repo,
            common_presenter,
        }
    }

    pub fn fetch_all_countries(&self, params: FetchAllCountriesUsecaseInput) -> Result<HttpResponse, AppError> {
        let countries = self.common_repo
            .get_all_countries(
                GetAllCountriesRepositoryInput {
                    limit: params.limit,
                    offset: params.offset,
                }
            )?;
        let response = self.common_presenter
            .to_multi_country_json(countries);

        Ok(response)
    }

    pub fn fetch_organisation_types(&self) -> Result<HttpResponse, AppError> {
        let org_types = self.common_repo
            .get_all_organisation_types()?;
        let response = self.common_presenter
            .to_multi_organization_type_json(org_types);

        Ok(response)
    }
}

pub struct FetchAllCountriesUsecaseInput {
    pub limit: i64,
    pub offset: i64,
}
