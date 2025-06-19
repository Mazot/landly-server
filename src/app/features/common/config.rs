use actix_web::{web, web::ServiceConfig};
use super::controllers::{fetch_all_countries};

pub fn configure_services(cfg: &mut ServiceConfig) -> () {
    cfg.service(
        web::scope("/common")
            .route("/countries", web::get()
                .to(fetch_all_countries))
    );
}
