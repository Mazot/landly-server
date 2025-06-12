use super::entities::{CreateOrganisation, Organisation};
use crate::error::AppError;
use crate::utils::db::DbPool;
use uuid::Uuid;

pub trait OrganisationRepository: Send + Sync + 'static {
    fn fetch_organisations(
        &self,
        params: FetchOrganisationsRepositoryInput
    ) -> Result<Vec<Organisation>, AppError>;

    fn create_organisation(
        &self,
        params: CreateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError>;
}

#[derive(Clone)]
pub struct OrganisationRepositoryImpl {
    pool: DbPool
}
impl OrganisationRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl OrganisationRepository for OrganisationRepositoryImpl {
    fn fetch_organisations(&self, params: FetchOrganisationsRepositoryInput) -> Result<Vec<Organisation>, AppError> {
        todo!()
    }

    fn create_organisation(&self, params: CreateOrganisationRepositoryInput) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;

        let new_organisation = Organisation::create(
            connection,
            &CreateOrganisation {
                name: params.name,
                tel: params.tel,
                email: params.email,
                address: params.address,
                description: params.description,
                location_country_id: params.location_country_id,
                organisation_type_id: params.organisation_type_id,
            }
        )?;

        Ok(new_organisation)
    }
}

pub struct FetchOrganisationsRepositoryInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub address: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub struct CreateOrganisationRepositoryInput {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
}
