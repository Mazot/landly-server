use super::controllers::{
    add_languages, delete_language, fetch_languages, fetch_me, oauth_google_callback,
    oauth_google_exchange, oauth_google_login, signin, signup, update_me,
    update_notification_settings,
};
use crate::app::drivers::middlewares::rate_limit::RateLimitConfig;
use actix_governor::Governor;
use actix_web::{web, web::ServiceConfig};

/// `auth_limit` must be built once at startup and shared by all workers.
pub fn create_configure_services_closure(
    auth_limit: RateLimitConfig,
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| configure_services(cfg, &auth_limit)
}

fn configure_services(cfg: &mut ServiceConfig, auth_limit: &RateLimitConfig) {
    // Credential endpoints get the strict per-client limiter (brute force,
    // mass signup, OAuth code spraying).
    cfg.service(
        web::scope("/user")
            .route(
                "/signin",
                web::post().to(signin).wrap(Governor::new(auth_limit)),
            )
            .route(
                "/signup",
                web::post().to(signup).wrap(Governor::new(auth_limit)),
            )
            .route(
                "/oauth/google/login",
                web::get()
                    .to(oauth_google_login)
                    .wrap(Governor::new(auth_limit)),
            )
            .route(
                "/oauth/google/callback",
                web::get()
                    .to(oauth_google_callback)
                    .wrap(Governor::new(auth_limit)),
            )
            .route(
                "/oauth/google/exchange",
                web::post()
                    .to(oauth_google_exchange)
                    .wrap(Governor::new(auth_limit)),
            )
            .route("/me", web::get().to(fetch_me))
            .route("/me", web::put().to(update_me))
            .route(
                "/me/notifications",
                web::put().to(update_notification_settings),
            )
            .route("/{user_id}/languages", web::get().to(fetch_languages))
            .route("/languages", web::post().to(add_languages))
            .route("/languages", web::delete().to(delete_language)),
    );
}
