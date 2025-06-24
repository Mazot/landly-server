use super::entities::{CountryConnection, CreateCountryConnection, UpdateCountryConnection};
use crate::error::AppError;
use crate::utils::db::DbPool;
use uuid::Uuid;

pub trait CountryConnectionRepository: Send + Sync + 'static {
    fn fetch_country_connections(
        &self,
        params: FetchCountryConnectionsRepositoryInput
    ) -> Result<Vec<CountryConnection>, AppError>;

    fn fetch_country_connection(
        &self,
        id: Uuid
    ) -> Result<CountryConnection, AppError>;

    fn create_country_connection(
        &self,
        params: CreateCountryConnectionRepositoryInput
    ) -> Result<CountryConnection, AppError>;

    fn update_country_connection(
        &self,
        id: Uuid,
        params: UpdateCountryConnectionRepositoryInput
    ) -> Result<CountryConnection, AppError>;

    fn delete_country_connection(
        &self,
        id: Uuid
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct CountryConnectionRepositoryImpl {
    pool: DbPool
}
impl CountryConnectionRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl CountryConnectionRepository for CountryConnectionRepositoryImpl {
    fn fetch_country_connections(&self, params: FetchCountryConnectionsRepositoryInput) -> Result<Vec<CountryConnection>, AppError> {
        let connection = &mut self.pool.get()?;
        let country_connections = CountryConnection::fetch_with_filters(
            connection,
            params.embassy_org_id,
            params.consulate_org_id,
            params.location_country_id,
            params.limit,
            params.offset,
        )?;

        Ok(country_connections)
    }

    fn create_country_connection(&self, params: CreateCountryConnectionRepositoryInput) -> Result<CountryConnection, AppError> {
        let connection = &mut self.pool.get()?;
        let new_country_connection = CountryConnection::create(
            connection,
            &CreateCountryConnection {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            }
        )?;

        Ok(new_country_connection)
    }

    fn delete_country_connection(&self, id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        CountryConnection::delete(connection, id)?;

        Ok(())
    }

    fn update_country_connection(
        &self,
        id: Uuid,
        params: UpdateCountryConnectionRepositoryInput
    ) -> Result<CountryConnection, AppError> {
        let connection = &mut self.pool.get()?;
        let updated_country_connection = CountryConnection::update(
            connection,
            id,
            &UpdateCountryConnection {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            }
        )?;

        Ok(updated_country_connection)
    }

    fn fetch_country_connection(
        &self,
        id: Uuid
    ) -> Result<CountryConnection, AppError> {
        let connection = &mut self.pool.get()?;
        let result = CountryConnection::fetch_by_id(connection, id)?;

        Ok(result)
    }
}

pub struct UpdateCountryConnectionRepositoryInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

pub struct FetchCountryConnectionsRepositoryInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub location_country_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct CreateCountryConnectionRepositoryInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}
