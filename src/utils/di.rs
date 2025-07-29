use super::{db::DbPool, redis::RedisPool};
use crate::app::features::organisation::{
    presenters::OrganisationPresenterImpl,
    repositories::OrganisationRepositoryImpl,
    usecases::OrganisationUsecase,
};
use crate::app::features::common::{
    presenters::CommonPresenterImpl,
    repositories::CommonRepositoryImpl,
    usecases::CommonUsecase,
};
use crate::app::features::country_connection::{
    repositories::CountryConnectionRepositoryImpl,
    presenters::CountryConnectionPresenterImpl,
    usecases::CountryConnectionUsecase,
};
use crate::utils::cache::{
    CacheService,
    NoOpCacheService,
    RedisCacheService,
    TypedCache
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
    pub common_usecase: CommonUsecase,
    pub country_connection_usecase: CountryConnectionUsecase,
    pub redis_cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl DiContainer {
    pub fn new(pool: &DbPool, redis_pool: Option<RedisPool>) -> Self {
        let typed_cache_service: TypedCache<Arc<dyn CacheService>> = TypedCache::new(
            match redis_pool {
                Some(pool) => Arc::new(RedisCacheService::new(pool)),
                None => Arc::new(NoOpCacheService::default()),
            }
        );

        let organisation_repo = OrganisationRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let organisation_presenter = OrganisationPresenterImpl::new();

        let common_repo = CommonRepositoryImpl::new(pool.clone());
        let common_presenter = CommonPresenterImpl::new();

        let country_connection_repo = CountryConnectionRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let country_connection_presenter = CountryConnectionPresenterImpl::new();

        Self {
            redis_cache_service: typed_cache_service.clone(),
            organisation_usecase: OrganisationUsecase::new(
                Arc::new(organisation_repo.clone()),
                Arc::new(organisation_presenter.clone()),
            ),
            common_usecase: CommonUsecase::new(
                Arc::new(common_repo.clone()),
                Arc::new(common_presenter.clone()),
            ),
            country_connection_usecase: CountryConnectionUsecase::new(
                Arc::new(country_connection_repo.clone()),
                Arc::new(country_connection_presenter.clone()),
            ),
        }
    }
}
