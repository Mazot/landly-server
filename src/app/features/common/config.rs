use super::controllers::{
    create_organisation_type, fetch_all_countries, fetch_all_organisation_types,
    fetch_country_detail,
};
use crate::utils::redis::make_common_get_request_cache;
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    let countries_cache = make_common_get_request_cache("common:countries:", 60 * 60);
    // TODO: Need to add invalidation for post and put requests.
    // Maybe need to patch lib. https://github.com/densumesh/actix-request-reply-cache/tree/main?tab=readme-ov-file
    let org_types_cache = make_common_get_request_cache("common:organisation_types:", 60 * 60);

    cfg.service(
        web::scope("/common")
            .route(
                "/countries",
                web::get().to(fetch_all_countries).wrap(countries_cache),
            )
            .route("/countries/{id}", web::get().to(fetch_country_detail))
            .route(
                "/org_types",
                web::get()
                    .to(fetch_all_organisation_types)
                    .wrap(org_types_cache),
            )
            .route("/org_types", web::post().to(create_organisation_type)),
    );
}
