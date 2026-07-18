use super::{db::DbPool, redis::RedisPool};
use crate::app::features::common::{
    presenters::CommonPresenterImpl, repositories::CommonRepositoryImpl, usecases::CommonUsecase,
};
use crate::app::features::corridor::{
    presenters::CorridorPresenterImpl, repositories::CorridorRepositoryImpl,
    usecases::CorridorUsecase,
};
use crate::app::features::country_connection::{
    presenters::CountryConnectionPresenterImpl, repositories::CountryConnectionRepositoryImpl,
    usecases::CountryConnectionUsecase,
};
use crate::app::features::images::{
    presenters::ImagePresenterImpl, repositories::ImageRepositoryImpl, usecases::ImageUsecase,
};
use crate::app::features::organisation::{
    presenters::OrganisationPresenterImpl, repositories::OrganisationRepositoryImpl,
    usecases::OrganisationUsecase,
};
use crate::app::features::user::{
    oauth::google::OAuthGoogle, presenters::UserPresenterImpl, repositories::UserRepositoryImpl,
    usecases::UserUsecase,
};
use crate::utils::cache::{CacheService, NoOpCacheService, RedisCacheService, TypedCache};
use crate::utils::s3::S3ClientWrapper;
use crate::utils::storage::{NoOpStorageService, StorageService};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
    pub common_usecase: CommonUsecase,
    pub corridor_usecase: CorridorUsecase,
    pub country_connection_usecase: CountryConnectionUsecase,
    pub user_usecase: UserUsecase,
    pub image_usecase: ImageUsecase,
    pub redis_cache_service: TypedCache<Arc<dyn CacheService>>,
    pub storage_service: Arc<dyn StorageService>,
    pub oauth_google: OAuthGoogle,
}

impl DiContainer {
    pub fn new(pool: &DbPool, redis_pool: Option<RedisPool>) -> Self {
        let typed_cache_service: TypedCache<Arc<dyn CacheService>> =
            TypedCache::new(match redis_pool {
                Some(pool) => Arc::new(RedisCacheService::new(pool)),
                None => Arc::new(NoOpCacheService),
            });

        // ---------------------------------------------------------------------------
        // Storage backend — S3 / Cloudflare R2 when env vars are present, NoOp
        // otherwise (upload / delete calls will return an error at runtime).
        // ---------------------------------------------------------------------------
        let storage_service: Arc<dyn StorageService> = match S3ClientWrapper::new() {
            Ok(svc) => {
                log::info!("Object storage backend initialised (S3/R2)");
                Arc::new(svc)
            }
            Err(_) => {
                log::warn!(
                    "Object storage is not configured (missing S3_* env vars). \
                     Image upload/delete endpoints will return 500 until storage is configured."
                );
                Arc::new(NoOpStorageService)
            }
        };

        // ---------------------------------------------------------------------------
        // Feature repositories
        // ---------------------------------------------------------------------------
        let organisation_repo =
            OrganisationRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let organisation_presenter = OrganisationPresenterImpl::new();

        let common_repo = CommonRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let common_presenter = CommonPresenterImpl::new();

        let corridor_repo = CorridorRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let corridor_presenter = CorridorPresenterImpl::new();

        let country_connection_repo =
            CountryConnectionRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let country_connection_presenter = CountryConnectionPresenterImpl::new();

        let user_repo = UserRepositoryImpl::new(pool.clone());
        let user_presenter = UserPresenterImpl::new();

        let image_repo = ImageRepositoryImpl::new(pool.clone(), typed_cache_service.clone());
        let image_presenter = ImagePresenterImpl::new();

        Self {
            redis_cache_service: typed_cache_service.clone(),
            storage_service: storage_service.clone(),

            organisation_usecase: OrganisationUsecase::new(
                Arc::new(organisation_repo.clone()),
                Arc::new(organisation_presenter.clone()),
            ),
            common_usecase: CommonUsecase::new(
                Arc::new(common_repo.clone()),
                Arc::new(common_presenter.clone()),
            ),
            corridor_usecase: CorridorUsecase::new(
                Arc::new(corridor_repo.clone()),
                Arc::new(corridor_presenter.clone()),
            ),
            country_connection_usecase: CountryConnectionUsecase::new(
                Arc::new(country_connection_repo.clone()),
                Arc::new(country_connection_presenter.clone()),
            ),
            user_usecase: UserUsecase::new(
                Arc::new(user_repo.clone()),
                Arc::new(user_presenter.clone()),
            ),
            image_usecase: ImageUsecase::new(
                Arc::new(image_repo),
                Arc::new(image_presenter),
                storage_service,
            ),
            oauth_google: OAuthGoogle::new(typed_cache_service.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::cache::NoOpCacheService;

    #[test]
    fn test_di_container_with_no_redis() {
        // This test doesn't require actual database connection
        // We're just testing that DiContainer can be constructed with NoOp cache
        let no_op_cache = TypedCache::new(Arc::new(NoOpCacheService) as Arc<dyn CacheService>);

        // Test that TypedCache can be cloned
        let cloned_cache = no_op_cache.clone();
        assert!(cloned_cache.exists("test").is_ok());
    }

    #[test]
    fn test_cache_service_selection_with_none() {
        let typed_cache_service: TypedCache<Arc<dyn CacheService>> =
            TypedCache::new(match None::<RedisPool> {
                Some(pool) => Arc::new(RedisCacheService::new(pool)),
                None => Arc::new(NoOpCacheService),
            });

        // Should use NoOpCacheService when redis_pool is None
        let result = typed_cache_service.exists("test");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_typed_cache_clone_in_di_context() {
        let cache_service: TypedCache<Arc<dyn CacheService>> =
            TypedCache::new(Arc::new(NoOpCacheService));

        let cache_clone1 = cache_service.clone();
        let cache_clone2 = cache_service.clone();

        // All clones should work independently
        assert!(cache_clone1.exists("key1").is_ok());
        assert!(cache_clone2.exists("key2").is_ok());
    }

    #[test]
    fn test_no_op_storage_service_get_public_url() {
        let storage = NoOpStorageService;
        let url = storage.get_public_url("images/test/file.jpg");
        assert_eq!(url, "images/test/file.jpg");
    }
}
