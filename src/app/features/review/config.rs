use super::controllers::{create_review, delete_review, list_reviews, review_summary};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/review")
            .route("/create", web::post().to(create_review))
            .route("/list", web::get().to(list_reviews))
            .route("/summary", web::get().to(review_summary))
            .route("/delete/{id}", web::delete().to(delete_review)),
    );
}
