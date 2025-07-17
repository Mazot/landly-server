use super::controllers::fetch_all_countries;
use crate::utils::redis::make_common_get_request_cache;
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) -> () {
    let countries_cache = make_common_get_request_cache(
        "common:countries:",
        60 * 60
    );

    cfg.service(
        web::scope("/common")
            .route("/countries", web::get()
                .to(fetch_all_countries))
                .wrap(countries_cache)
    );
}
