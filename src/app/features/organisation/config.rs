use super::controllers::{
    create_organisation, delete_organisation, fetch_organisation, list_organisations,
    search_organisations, update_organisation, visit_organisation,
};
use crate::utils::cache::{CacheService, TypedCache};
use actix_web::{web, web::ServiceConfig};
use std::sync::Arc;

pub fn create_configure_services_closure(
    middleware: TypedCache<Arc<dyn CacheService>>,
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/organisation")
                .wrap(middleware.clone())
                .route("/create", web::post().to(create_organisation))
                .route("/list", web::get().to(list_organisations))
                .route("/search", web::get().to(search_organisations))
                .route("/delete/{id}", web::delete().to(delete_organisation))
                .route("/update/{id}", web::put().to(update_organisation))
                .route("/fetch/{id}", web::get().to(fetch_organisation))
                .route("/visit/{id}", web::post().to(visit_organisation)),
        );
    }
}
