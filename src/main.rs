#![allow(dead_code)]
mod app;
mod constants;
pub mod data;
mod error;
pub mod utils;

use crate::app::drivers::middlewares::{auth::Authentication, cors::cors};
use actix_web::middleware::Logger;
use actix_web::{App, HttpResponse, HttpServer, web};
use dotenv::dotenv;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Landly Web API",
        description = "Landly OpenAPI Specification."
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.orsoft.xyz", description = "Production server"),
    )
)]
pub struct ApiDoc;

/// Root doc (legacy features registered above) merged with the per-feature
/// `ApiDoc`s of the phase-2 modules. New features must declare their own doc
/// in `<feature>/mod.rs` and be merged here instead of growing the root macro.
pub fn build_openapi() -> utoipa::openapi::OpenApi {
    let mut openapi = ApiDoc::openapi();

    openapi.merge(app::features::healthcheck::ApiDoc::openapi());
    openapi.merge(app::features::common::ApiDoc::openapi());
    openapi.merge(app::features::organisation::ApiDoc::openapi());
    openapi.merge(app::features::country_connection::ApiDoc::openapi());
    openapi.merge(app::features::user::ApiDoc::openapi());
    openapi.merge(app::features::corridor::ApiDoc::openapi());
    openapi.merge(app::features::images::ApiDoc::openapi());
    openapi.merge(app::features::person::ApiDoc::openapi());
    openapi.merge(app::features::review::ApiDoc::openapi());
    openapi.merge(app::features::saved::ApiDoc::openapi());
    openapi.merge(app::features::report::ApiDoc::openapi());
    openapi.merge(app::features::moderation::ApiDoc::openapi());

    openapi
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("start server");
    // std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let app_state = {
        use crate::app::drivers::middlewares::state::AppState;
        let db_pool = utils::db::establish_connection();
        AppState::new(db_pool)
    };

    HttpServer::new(move || {
        let country_connection_configure_services =
            app::features::country_connection::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );
        let organisation_configure_services =
            app::features::organisation::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );
        let images_configure_services =
            app::features::images::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(Logger::default())
            .wrap(cors())
            .wrap(Authentication)
            // API docs: Scalar UI (replaces the heavier Swagger UI, which
            // embedded the whole swagger dist via rust-embed at build time).
            .service(Scalar::with_url("/scalar", build_openapi()))
            // Raw spec for codegen/tooling, same path as before.
            .route(
                "/api-docs/openapi.json",
                web::get().to(|| async { HttpResponse::Ok().json(build_openapi()) }),
            )
            // Old bookmark compatibility.
            .service(web::redirect("/swagger-ui", "/scalar"))
            .service(web::redirect("/swagger-ui/", "/scalar"))
            .service(
                web::scope("/api")
                    .service(web::scope("/healthcheck").route(
                        "",
                        web::get().to(app::features::healthcheck::controllers::index),
                    ))
                    .configure(app::features::common::config::configure_services)
                    .configure(app::features::user::config::configure_services)
                    .configure(app::features::corridor::config::configure_services)
                    .configure(app::features::person::config::configure_services)
                    .configure(app::features::review::config::configure_services)
                    .configure(app::features::saved::config::configure_services)
                    .configure(app::features::report::config::configure_services)
                    .configure(app::features::moderation::config::configure_services)
                    .configure(organisation_configure_services)
                    .configure(country_connection_configure_services)
                    .configure(images_configure_services),
            )
    })
    .bind(constants::BIND)?
    .run()
    .await
}
