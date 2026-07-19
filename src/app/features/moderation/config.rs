use super::controllers::{approve, fetch_queue, reject, request_changes};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/moderation")
            .route("/queue", web::get().to(fetch_queue))
            .route("/approve", web::post().to(approve))
            .route("/request-changes", web::post().to(request_changes))
            .route("/reject", web::post().to(reject)),
    );
}
