use super::controllers::{counts_saved, create_saved, delete_saved, list_saved};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/saved")
            .route("/create", web::post().to(create_saved))
            .route("/delete/{id}", web::delete().to(delete_saved))
            .route("/list", web::get().to(list_saved))
            .route("/counts", web::get().to(counts_saved)),
    );
}
