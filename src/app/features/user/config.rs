use super::controllers::{
    add_languages, delete_language, fetch_languages, fetch_me, oauth_google_callback,
    oauth_google_login, signin, signup, update_me, update_notification_settings,
};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/signin", web::post().to(signin))
            .route("/signup", web::post().to(signup))
            .route("/oauth/google/login", web::get().to(oauth_google_login))
            .route(
                "/oauth/google/callback",
                web::get().to(oauth_google_callback),
            )
            .route("/me", web::get().to(fetch_me))
            .route("/me", web::put().to(update_me))
            .route(
                "/me/notifications",
                web::put().to(update_notification_settings),
            )
            .route("{user_id}/languages", web::get().to(fetch_languages))
            .route("/languages", web::post().to(add_languages))
            .route("/languages", web::delete().to(delete_language)),
    );
}
