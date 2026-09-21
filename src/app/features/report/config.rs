use super::controllers::create_report;
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(web::scope("/report").route("/create", web::post().to(create_report)));
}
