use super::controllers::{create, delete, fetch, list, update};
use crate::utils::cache::{CacheService, TypedCache};
use actix_web::{web, web::ServiceConfig};
use std::sync::Arc;

pub fn create_configure_services_closure(
    middleware: TypedCache<Arc<dyn CacheService>>,
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/country-connection")
                .wrap(middleware.clone())
                .route("/create", web::post().to(create))
                .route("/list", web::get().to(list))
                .route("/delete/{id}", web::delete().to(delete))
                .route("/update/{id}", web::put().to(update))
                .route("/fetch/{id}", web::get().to(fetch)),
        );
    }
}
