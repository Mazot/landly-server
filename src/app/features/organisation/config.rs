use super::controllers::{
    checkin_organisation, create_organisation, delete_organisation, fetch_organisation,
    list_organisations, search_organisations, update_organisation, visit_organisation,
};
use crate::app::drivers::middlewares::rate_limit::{RateLimitConfig, UserRateLimitConfig};
use crate::utils::cache::{CacheService, TypedCache};
use actix_governor::Governor;
use actix_web::{web, web::ServiceConfig};
use std::sync::Arc;

/// `submit_ip` / `submit_user` throttle `POST /create` (a Google Places
/// lookup is billed and every submission lands in the moderation queue).
/// Both must be built once at startup and shared by all workers.
pub fn create_configure_services_closure(
    middleware: TypedCache<Arc<dyn CacheService>>,
    submit_ip: RateLimitConfig,
    submit_user: UserRateLimitConfig,
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/organisation")
                .wrap(middleware.clone())
                .route(
                    "/create",
                    // The last `wrap` is the outermost: a request over the IP
                    // limit is refused before it is counted against the user.
                    web::post()
                        .to(create_organisation)
                        .wrap(Governor::new(&submit_user))
                        .wrap(Governor::new(&submit_ip)),
                )
                .route("/list", web::get().to(list_organisations))
                .route("/search", web::get().to(search_organisations))
                .route("/delete/{id}", web::delete().to(delete_organisation))
                .route("/update/{id}", web::put().to(update_organisation))
                .route("/fetch/{id}", web::get().to(fetch_organisation))
                .route("/visit/{id}", web::post().to(visit_organisation))
                .route("/checkin/{id}", web::post().to(checkin_organisation)),
        );
    }
}
