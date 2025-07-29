use super::entities::{CountryConnection, CreateCountryConnection, UpdateCountryConnection};
use crate::{
    error::AppError,
    utils::{cache::{CacheKeys, CacheService, TypedCache}, db::DbPool}
};
use std::{
    collections::hash_map::DefaultHasher,
    time::Duration,
    hash::{Hash, Hasher},
    sync::Arc,
};
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
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}
impl CountryConnectionRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self { pool, cache_service }
    }

    fn generate_filters_hash(params: &FetchCountryConnectionsRepositoryInput) -> String {
        let mut hasher = DefaultHasher::new();
        params.embassy_org_id.hash(&mut hasher);
        params.consulate_org_id.hash(&mut hasher);
        params.location_country_id.hash(&mut hasher);
        params.limit.hash(&mut hasher);
        params.offset.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }
}

impl CountryConnectionRepository for CountryConnectionRepositoryImpl {
    fn fetch_country_connections(&self, params: FetchCountryConnectionsRepositoryInput) -> Result<Vec<CountryConnection>, AppError> {
        let filters_hash = Self::generate_filters_hash(&params);
        let cache_key = CacheKeys::country_connections_list(&filters_hash);

        if let Some(cached_c_c) = self.cache_service.get::<Vec<CountryConnection>>(&cache_key)? {
            return Ok(cached_c_c);
        };

        let connection = &mut self.pool.get()?;
        let country_connections = CountryConnection::fetch_with_filters(
            connection,
            params.embassy_org_id,
            params.consulate_org_id,
            params.location_country_id,
            params.limit,
            params.offset,
        )?;

        let _ = self.cache_service.set::<Vec<CountryConnection>>(
            &cache_key,
            &country_connections,
            Some(Duration::from_secs(5 * 60))
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

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::country_connection_pattern())?;

        let cache_key = CacheKeys::country_connection_by_id(&new_country_connection.id);
        let _ = self.cache_service.set::<CountryConnection>(
            &cache_key,
            &new_country_connection,
            None
        )?;

        Ok(new_country_connection)
    }

    fn delete_country_connection(&self, id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        CountryConnection::delete(connection, id)?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::country_connection_pattern())?;

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

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::country_connection_pattern())?;

        let cache_key = CacheKeys::country_connection_by_id(&updated_country_connection.id);
        let _ = self.cache_service.set::<CountryConnection>(
            &cache_key,
            &updated_country_connection,
            None
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
