use super::controllers::{delete_image, fetch_image, list_images, set_primary_image, upload_image};
use crate::utils::cache::{CacheService, TypedCache};
use actix_web::{web, web::ServiceConfig};
use std::sync::Arc;

pub fn create_configure_services_closure(
    middleware: TypedCache<Arc<dyn CacheService>>,
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/images")
                .wrap(middleware.clone())
                .route("/upload/{organisation_id}", web::post().to(upload_image))
                .route("/delete/{id}", web::delete().to(delete_image))
                .route("/list/{organisation_id}", web::get().to(list_images))
                .route("/fetch/{id}", web::get().to(fetch_image))
                .route("/set-primary/{id}", web::put().to(set_primary_image)),
        );
    }
}
