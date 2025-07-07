use super::entities::{CreateOrganisation, Organisation, UpdateOrganisation};
use crate::error::AppError;
use crate::utils::db::DbPool;
use uuid::Uuid;
use bigdecimal::BigDecimal;

pub trait OrganisationRepository: Send + Sync + 'static {
    fn fetch_organisations(
        &self,
        params: FetchOrganisationsRepositoryInput
    ) -> Result<Vec<Organisation>, AppError>;

    fn create_organisation(
        &self,
        params: CreateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError>;

    fn delete_organisation(
        &self,
        id: Uuid
    ) -> Result<(), AppError>;

    fn fetch_organisation(
        &self,
        id: Uuid
    ) -> Result<Organisation, AppError>;

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput
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
        use crate::data::schema::organisations;
        use diesel::prelude::*;

        let connection = &mut self.pool.get()?;
        let query = {
            let mut query = organisations::table.into_boxed();

            if let Some(name) = &params.name {
                query = query.filter(organisations::name.ilike(format!("%{}%", name)));
            }

            if let Some(tel) = &params.tel {
                query = query.filter(organisations::tel.ilike(format!("%{}%", tel)));
            }

            if let Some(email) = &params.email {
                query = query.filter(organisations::email.ilike(format!("%{}%", email)));
            }

            if let Some(address) = &params.address {
                query = query.filter(organisations::address.ilike(format!("%{}%", address)));
            }

            if let Some(location_country_id) = params.location_country_id {
                let ids = Organisation::fetch_ids_by_location_country(connection, location_country_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            if let Some(organisation_type_id) = params.organisation_type_id {
                let ids = Organisation::fetch_ids_by_organisation_type(connection, organisation_type_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            query
        };

        let organisations = query
            .limit(params.limit)
            .offset(params.offset)
            .load::<Organisation>(connection)?;

        Ok(organisations)
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
                latitude: params.latitude,
                longitude: params.longitude,
            }
        )?;

        Ok(new_organisation)
    }

    fn delete_organisation(&self, id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        Organisation::delete(connection, id)?;

        Ok(())
    }

    fn fetch_organisation(&self, id: Uuid) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;
        let organisation = Organisation::fetch_by_id(connection, id)?;

        Ok(organisation)
    }

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;
        let updated_organisation = Organisation::update(
            connection,
            id,
            &UpdateOrganisation {
                name: params.name,
                tel: params.tel,
                email: params.email,
                address: params.address,
                description: params.description,
                location_country_id: params.location_country_id,
                organisation_type_id: params.organisation_type_id,
                longitude: params.longitude,
                latitude: params.latitude,
                updated_at: chrono::Utc::now().naive_utc(),
            }
        )?;

        Ok(updated_organisation)
    }
}

pub struct UpdateOrganisationRepositoryInput {
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

pub struct FetchOrganisationsRepositoryInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct CreateOrganisationRepositoryInput {
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
