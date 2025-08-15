use super::controllers::{signin, signup, add_languages, delete_language, fetch_languages};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/signin", web::post().to(signin))
            .route("/signup", web::post().to(signup))
            .route("{user_id}/languages", web::get().to(fetch_languages))
            .route("/languages", web::post().to(add_languages))
            .route("/languages", web::delete().to(delete_language)),
    );
}
