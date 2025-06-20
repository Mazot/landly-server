use std::sync::Arc;
use actix_web::HttpResponse;
use uuid::Uuid;
use crate::error::AppError;
use super::{
    presenters::OrganisationPresenter,
    repositories::OrganisationRepository,
    repositories::CreateOrganisationRepositoryInput,
};

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
                }
            )?;
        let response = self.organisation_presenter.to_single_json(new_organisation);

        Ok(response)
    }

    pub fn delete_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        self.organisation_repo
            .delete_organisation(id)?;
        let response = self.organisation_presenter.to_http_res();
        
        Ok(response)
    }
}

pub struct CreateOrganisationUsecaseInput {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
}
