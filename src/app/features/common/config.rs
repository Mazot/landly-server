use super::controllers::{
    create_organisation_type, fetch_all_countries, fetch_all_languages,
    fetch_all_organisation_types, fetch_country_detail,
};
use crate::utils::redis::make_common_get_request_cache;
use actix_web::{Route, web, web::ServiceConfig};

/// `redis_url` enables the request-reply cache on `GET /countries`,
/// `GET /languages` and `GET /org_types` (`common:*` keys, invalidated by
/// `CommonRepositoryImpl` on writes; languages have no API write path and
/// expire with the TTL).
/// Without Redis the routes are served uncached — REDIS_URL is optional.
pub fn create_configure_services_closure(redis_url: Option<String>) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| configure_services(cfg, redis_url.as_deref())
}

/// Wraps `route` in the response cache when Redis is configured.
fn cached(route: Route, redis_url: Option<&str>, prefix: &str) -> Route {
    match redis_url {
        Some(url) => route.wrap(make_common_get_request_cache(url, prefix, 60 * 60)),
        None => route,
    }
}

fn configure_services(cfg: &mut ServiceConfig, redis_url: Option<&str>) {
    // TODO: Need to add invalidation for post and put requests.
    // Maybe need to patch lib. https://github.com/densumesh/actix-request-reply-cache/tree/main?tab=readme-ov-file
    cfg.service(
        web::scope("/common")
            .route(
                "/countries",
                cached(
                    web::get().to(fetch_all_countries),
                    redis_url,
                    "common:countries:",
                ),
            )
            .route("/countries/{id}", web::get().to(fetch_country_detail))
            .route(
                "/languages",
                cached(
                    web::get().to(fetch_all_languages),
                    redis_url,
                    "common:languages:",
                ),
            )
            .route(
                "/org_types",
                cached(
                    web::get().to(fetch_all_organisation_types),
                    redis_url,
                    "common:organisation_types:",
                ),
            )
            .route("/org_types", web::post().to(create_organisation_type)),
    );
}
