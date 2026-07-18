use super::{
    presenters::CommonPresenter,
    repositories::{CommonRepository, GetAllCountriesRepositoryInput},
};
use crate::{
    app::features::common::repositories::CreateOrganisationTypeRepositoryInput, error::AppError,
};
use actix_web::HttpResponse;
use std::sync::Arc;

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

    pub fn fetch_all_countries(
        &self,
        params: FetchAllCountriesUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let countries = self
            .common_repo
            .get_all_countries(GetAllCountriesRepositoryInput {
                limit: params.limit,
                offset: params.offset,
                name: params.name,
            })?;
        let response = self.common_presenter.to_multi_country_json(countries);

        Ok(response)
    }

    pub fn fetch_country_detail(&self, id: uuid::Uuid) -> Result<HttpResponse, AppError> {
        let (country, by_type) = self.common_repo.get_country_detail(id)?;
        let response = self
            .common_presenter
            .to_country_detail_json(country, by_type);

        Ok(response)
    }

    pub fn fetch_organisation_types(&self) -> Result<HttpResponse, AppError> {
        let org_types = self.common_repo.get_all_organisation_types()?;
        let response = self
            .common_presenter
            .to_multi_organization_type_json(org_types);

        Ok(response)
    }

    pub fn create_organisation_type(
        &self,
        params: CreateOrganisationTypeUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let org_type =
            self.common_repo
                .create_organisation_type(CreateOrganisationTypeRepositoryInput {
                    org_type: params.org_type,
                    color: params.color,
                    title: params.title,
                    slug: params.slug,
                })?;
        let response = self
            .common_presenter
            .to_single_organization_type_json(org_type);

        Ok(response)
    }
}

pub struct FetchAllCountriesUsecaseInput {
    pub limit: i64,
    pub offset: i64,
    pub name: Option<String>,
}

pub struct CreateOrganisationTypeUsecaseInput {
    pub org_type: String,
    pub color: String,
    pub title: String,
    pub slug: Option<String>,
}
