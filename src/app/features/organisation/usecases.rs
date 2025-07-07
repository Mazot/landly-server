use crate::error::AppError;
use super::{
    presenters::OrganisationPresenter,
    repositories::{CreateOrganisationRepositoryInput, FetchOrganisationsRepositoryInput, OrganisationRepository, UpdateOrganisationRepositoryInput},
};
use std::sync::Arc;
use actix_web::HttpResponse;
use bigdecimal::BigDecimal;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrganisationUsecase {
    organisation_repo: Arc<dyn OrganisationRepository>,
    organisation_presenter: Arc<dyn OrganisationPresenter>,
}

impl OrganisationUsecase {
    pub fn new(
        organisation_repo: Arc<dyn OrganisationRepository>,
        organisation_presenter: Arc<dyn OrganisationPresenter>,
    ) -> Self {
        Self {
            organisation_repo,
            organisation_presenter,
        }
    }

    pub fn create_organisation(
        &self,
        params: CreateOrganisationUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let new_organisation = self.organisation_repo
            .create_organisation(
                CreateOrganisationRepositoryInput {
                    name: params.name,
                    tel: params.tel,
                    email: params.email,
                    address: params.address,
                    description: params.description,
                    location_country_id: params.location_country_id,
                    organisation_type_id: params.organisation_type_id,
                    latitude: params.latitude,
                    longitude: params.longitude,
                }
            )?;
        let response = self.organisation_presenter.to_single_json(new_organisation);

        Ok(response)
    }

    pub fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let updated_organisation = self.organisation_repo
            .update_organisation(
                id,
                UpdateOrganisationRepositoryInput {
                    name: params.name,
                    tel: params.tel,
                    email: params.email,
                    address: params.address,
                    description: params.description,
                    location_country_id: params.location_country_id,
                    organisation_type_id: params.organisation_type_id,
                    latitude: params.latitude,
                    longitude: params.longitude,
                }
            )?;
        let response = self.organisation_presenter.to_single_json(updated_organisation);

        Ok(response)
    }

    pub fn fetch_organisations(
        &self,
        params: FetchOrganisationsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let organisations = self.organisation_repo
            .fetch_organisations(
                FetchOrganisationsRepositoryInput {
                    name: params.name,
                    tel: params.tel,
                    email: params.email,
                    location_country_id: params.location_country_id,
                    organisation_type_id: params.organisation_type_id,
                    address: params.address,
                    limit: params.limit,
                    offset: params.offset,
                }
            )?;
        let response = self.organisation_presenter.to_multi_json(organisations);

        Ok(response)
    }

    pub fn delete_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        self.organisation_repo
            .delete_organisation(id)?;
        let response = self.organisation_presenter.to_http_res();

        Ok(response)
    }

    pub fn fetch_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let organisation = self.organisation_repo.fetch_organisation(id)?;
        let response = self.organisation_presenter.to_single_json(organisation);

        Ok(response)
    }
}

pub struct UpdateOrganisationUsecaseInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}

pub struct CreateOrganisationUsecaseInput {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}

pub struct FetchOrganisationsUsecaseInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}
