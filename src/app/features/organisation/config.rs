use actix_web::{web, web::ServiceConfig};
use super::controllers::{create};

pub fn configure_services(cfg: &mut ServiceConfig) -> () {
    cfg.service(
        web::scope("/organisation")
            .route("/create", web::post()
                .to(create))
            // .route("/update/{id}", web::put()
            //     .to(app::features::organisation::controllers::update))
            // .route("/delete/{id}", web::delete()
            //     .to(app::features::organisation::controllers::delete))
            // .route("/fetch_all", web::get()
            //     .to(app::features::organisation::controllers::fetch_all))
            // .route("/fetch/{id}", web::get()
            //     .to(app::features::organisation::controllers::fetch))
    );
}
