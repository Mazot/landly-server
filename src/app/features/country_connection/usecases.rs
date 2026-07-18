use super::{
    presenters::CountryConnectionPresenter,
    repositories::{
        CountryConnectionRepository, CreateCountryConnectionRepositoryInput,
        FetchCountryConnectionsRepositoryInput, UpdateCountryConnectionRepositoryInput,
    },
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CountryConnectionUsecase {
    country_connection_repo: Arc<dyn CountryConnectionRepository>,
    country_connection_presenter: Arc<dyn CountryConnectionPresenter>,
}

impl CountryConnectionUsecase {
    pub fn new(
        country_connection_repo: Arc<dyn CountryConnectionRepository>,
        country_connection_presenter: Arc<dyn CountryConnectionPresenter>,
    ) -> Self {
        Self {
            country_connection_repo,
            country_connection_presenter,
        }
    }

    /// Country connections are a system reference table — admin only.
    fn ensure_admin(&self, caller_user_id: Uuid) -> Result<(), AppError> {
        let role = self
            .country_connection_repo
            .fetch_user_role(caller_user_id)?;

        if role.is_admin() {
            return Ok(());
        }

        Err(AppError::Forbidden(
            json!({ "error": "Only an admin can manage country connections" }),
        ))
    }

    pub fn fetch_country_connection(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let country_connection = self.country_connection_repo.fetch_country_connection(id)?;
        let response = self
            .country_connection_presenter
            .to_single_json(country_connection);

        Ok(response)
    }

    pub fn fetch_country_connections(
        &self,
        params: FetchCountryConnectionsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let country_connections = self.country_connection_repo.fetch_country_connections(
            FetchCountryConnectionsRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                location_country_id: params.location_country_id,
                limit: params.limit,
                offset: params.offset,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_multi_json(country_connections);

        Ok(response)
    }

    pub fn create_country_connection(
        &self,
        caller_user_id: Uuid,
        params: CreateCountryConnectionUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        let new_country_connection = self.country_connection_repo.create_country_connection(
            CreateCountryConnectionRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_single_json(new_country_connection);

        Ok(response)
    }

    pub fn update_country_connection(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
        params: UpdateCountryConnectionUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        let updated_country_connection = self.country_connection_repo.update_country_connection(
            id,
            UpdateCountryConnectionRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_single_json(updated_country_connection);

        Ok(response)
    }

    pub fn delete_country_connection(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        self.country_connection_repo.delete_country_connection(id)?;
        let response = self.country_connection_presenter.to_http_res();

        Ok(response)
    }
}

pub struct FetchCountryConnectionsUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub location_country_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct CreateCountryConnectionUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

pub struct UpdateCountryConnectionUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}
