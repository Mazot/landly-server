use super::controllers::{create_organisation, list_organisations, delete_organisation, fetch_organisation, update_organisation};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) -> () {
    cfg.service(
        web::scope("/organisation")
            .route("/create", web::post()
                .to(create_organisation))
            .route("/list", web::get()
                .to(list_organisations))
            .route("/delete/{id}", web::delete()
                .to(delete_organisation))
            .route("/update/{id}", web::put()
                .to(update_organisation))
            .route("/fetch/{id}", web::get()
                .to(fetch_organisation))
    );
}
