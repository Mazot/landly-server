use super::controllers::{create_review, delete_review, list_reviews};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/review")
            .route("/create", web::post().to(create_review))
            .route("/list", web::get().to(list_reviews))
            .route("/delete/{id}", web::delete().to(delete_review)),
    );
}
