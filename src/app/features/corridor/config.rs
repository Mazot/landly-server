use super::controllers::{
    create_corridor, delete_corridor, fetch_corridor_stats, list_corridors, set_default_corridor,
};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/corridor")
            .route("/create", web::post().to(create_corridor))
            .route("/list", web::get().to(list_corridors))
            .route("/set-default/{id}", web::put().to(set_default_corridor))
            .route("/delete/{id}", web::delete().to(delete_corridor))
            .route("/stats/{id}", web::get().to(fetch_corridor_stats)),
    );
}
