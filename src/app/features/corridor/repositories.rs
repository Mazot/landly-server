use super::entities::{Corridor, CorridorStats, CreateCorridor};
use crate::{
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub trait CorridorRepository: Send + Sync + 'static {
    fn create_corridor(&self, params: CreateCorridorRepositoryInput) -> Result<Corridor, AppError>;

    fn list_corridors(&self, user_id: Uuid) -> Result<Vec<Corridor>, AppError>;

    fn set_default_corridor(&self, corridor_id: Uuid, user_id: Uuid) -> Result<Corridor, AppError>;

    fn delete_corridor(&self, corridor_id: Uuid, user_id: Uuid) -> Result<(), AppError>;

    fn fetch_corridor_stats(
        &self,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<CorridorStats, AppError>;
}

#[derive(Clone)]
pub struct CorridorRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl CorridorRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }
}

impl CorridorRepository for CorridorRepositoryImpl {
    fn create_corridor(&self, params: CreateCorridorRepositoryInput) -> Result<Corridor, AppError> {
        let connection = &mut self.pool.get()?;
        let corridor = Corridor::create(
            connection,
            &CreateCorridor {
                user_id: params.user_id,
                from_country_id: params.from_country_id,
                to_country_id: params.to_country_id,
                is_default: params.is_default,
            },
        )?;

        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::corridor_pattern());

        Ok(corridor)
    }

    fn list_corridors(&self, user_id: Uuid) -> Result<Vec<Corridor>, AppError> {
        let cache_key = CacheKeys::corridors_by_user(&user_id);

        if let Some(cached) = self.cache_service.get::<Vec<Corridor>>(&cache_key)? {
            return Ok(cached);
        }

        let connection = &mut self.pool.get()?;
        let corridors = Corridor::list_by_user(connection, user_id)?;

        let _ = self.cache_service.set::<Vec<Corridor>>(
            &cache_key,
            &corridors,
            Some(Duration::from_secs(5 * 60)),
        );

        Ok(corridors)
    }

    fn set_default_corridor(&self, corridor_id: Uuid, user_id: Uuid) -> Result<Corridor, AppError> {
        let connection = &mut self.pool.get()?;
        let corridor = Corridor::set_default(connection, corridor_id, user_id)?;

        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::corridor_pattern());

        Ok(corridor)
    }

    fn delete_corridor(&self, corridor_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        Corridor::delete(connection, corridor_id, user_id)?;

        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::corridor_pattern());

        Ok(())
    }

    fn fetch_corridor_stats(
        &self,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<CorridorStats, AppError> {
        let connection = &mut self.pool.get()?;
        // Ownership is always re-checked; only the counters themselves are cached.
        let corridor = Corridor::fetch_owned(connection, corridor_id, user_id)?;

        let cache_key = CacheKeys::corridor_stats(&corridor_id);
        if let Some(cached) = self.cache_service.get::<CorridorStats>(&cache_key)? {
            return Ok(cached);
        }

        let stats = Corridor::stats(connection, corridor)?;

        let _ = self.cache_service.set::<CorridorStats>(
            &cache_key,
            &stats,
            Some(Duration::from_secs(5 * 60)),
        );

        Ok(stats)
    }
}

pub struct CreateCorridorRepositoryInput {
    pub user_id: Uuid,
    pub from_country_id: Uuid,
    pub to_country_id: Uuid,
    pub is_default: bool,
}
