use super::controllers::{create, list, delete, fetch, update};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) -> () {
    cfg.service(
        web::scope("/country-connection")
            .route("/create", web::post()
                .to(create))
            .route("/list", web::get()
                .to(list))
            .route("/delete/{id}", web::delete()
                .to(delete))
            .route("/update/{id}", web::put()
                .to(update))
            .route("/fetch/{id}", web::get()
                .to(fetch))
    );
}
