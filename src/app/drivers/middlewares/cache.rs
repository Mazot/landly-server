use crate::{error::AppError, utils::cache::{CacheKeys, CacheService, TypedCache}};
use std::sync::Arc;
use std::future::{ready, Ready};
use futures::future::LocalBoxFuture;
use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, http::Method, Error};

pub struct CacheInvalidationMiddleware<S> {
    service: S,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl<S, B> Transform<S, ServiceRequest> for TypedCache<Arc<dyn CacheService>>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CacheInvalidationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CacheInvalidationMiddleware {
            service,
            cache_service: self.clone()
        }))
    }
}

impl<S, B> Service<ServiceRequest> for CacheInvalidationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Error = Error;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        println!("Hi from start. You requested: {}", req.path());

        let path = req.path().to_string();
        let method = req.method().clone();

        let cache_service = self.cache_service.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            if res.status().is_success() && (method == Method::POST || method == Method::PUT || method == Method::DELETE) {
                let _ = invalidate_cache_for_path(&cache_service, &method, &path).await;
            }

            println!("Hi from response");

            Ok(res)
        })
    }
}

async fn invalidate_cache_for_path(
    cache_service: &TypedCache<Arc<dyn CacheService>>,
    method: &Method,
    path: &str
) -> Result<(), AppError> {
    if method == Method::GET {
        return Ok(());
    }

    if path.contains("/organisation") {
        let _ = cache_service.invalidate_pattern(&CacheKeys::organisation_pattern());
        log::debug!("Invalidated organisation cache due to {} on {}", method, path);
    }

    if path.contains("/country-connection") {
        let _ = cache_service.invalidate_pattern(&CacheKeys::country_connection_pattern());
        log::debug!("Invalidated country connection cache due to {} on {}", method, path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::cache::NoOpCacheService;
    use actix_web::test::{self, *};

    #[actix_web::test]
    async fn no_op_cache_invalidation() {
        let no_op_service = NoOpCacheService::default();
        let typed_cache_mw = TypedCache::<Arc<dyn CacheService>>::new(
            Arc::new(no_op_service)
        )
            .new_transform(test::ok_service())
            .await
            .unwrap();

        let req = TestRequest::default().to_srv_request();
        let res = typed_cache_mw.call(req).await.unwrap();
        assert!(res.status().is_success());
    }
}
