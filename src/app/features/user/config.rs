use super::controllers::{signin, signup, add_languages, delete_language, fetch_languages, oauth_google_callback, oauth_google_login};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/signin", web::post().to(signin))
            .route("/signup", web::post().to(signup))
            .route("/oauth/google/login", web::get().to(oauth_google_login))
            .route("/oauth/google/callback", web::get().to(oauth_google_callback))
            .route("{user_id}/languages", web::get().to(fetch_languages))
            .route("/languages", web::post().to(add_languages))
            .route("/languages", web::delete().to(delete_language)),
    );
}
