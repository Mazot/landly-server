#![allow(dead_code)]
mod app;
mod constants;
pub mod data;
mod error;
pub mod utils;

use crate::app::drivers::middlewares::{auth::Authentication, cors::cors, rate_limit};
use actix_governor::Governor;
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

    // Built once at startup and shared (cheap clone) with every worker —
    // merging all the per-feature specs on every request to
    // /api-docs/openapi.json would be wasted work since the doc is static.
    let openapi = build_openapi();

    // Rate limiter state must be shared by all workers, so the configs are
    // built once here and only cloned into each worker's App.
    let api_rate_limit = rate_limit::api_rate_limit();
    let auth_rate_limit = rate_limit::auth_rate_limit();

    HttpServer::new(move || {
        let country_connection_configure_services =
            app::features::country_connection::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );
        let organisation_configure_services =
            app::features::organisation::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );
        let user_configure_services =
            app::features::user::config::create_configure_services_closure(auth_rate_limit.clone());
        let images_configure_services =
            app::features::images::config::create_configure_services_closure(
                app_state.di_container.redis_cache_service.clone(),
            );

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(openapi.clone()))
            .wrap(Logger::default())
            // Inside CORS so that 429 responses still carry CORS headers.
            .wrap(Governor::new(&api_rate_limit))
            .wrap(cors())
            .wrap(Authentication)
            // API docs: Scalar UI (replaces the heavier Swagger UI, which
            // embedded the whole swagger dist via rust-embed at build time).
            .service(Scalar::with_url("/scalar", openapi.clone()))
            // Raw spec for codegen/tooling, same path as before. Reuses the
            // doc built once at startup instead of rebuilding it per request.
            .route(
                "/api-docs/openapi.json",
                web::get().to(|doc: web::Data<utoipa::openapi::OpenApi>| async move {
                    HttpResponse::Ok().json(doc.get_ref())
                }),
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
                    .configure(user_configure_services)
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

#[cfg(test)]
mod tests {
    use super::build_openapi;
    use crate::app::route_inventory::{normalize_params, registered_routes};
    use std::collections::BTreeSet;

    /// (METHOD, normalized path) of every operation in the merged spec.
    fn documented_operations() -> BTreeSet<(String, String)> {
        let mut ops = BTreeSet::new();

        for (path, item) in build_openapi().paths.paths {
            let methods = [
                ("GET", item.get.is_some()),
                ("POST", item.post.is_some()),
                ("PUT", item.put.is_some()),
                ("DELETE", item.delete.is_some()),
                ("PATCH", item.patch.is_some()),
            ];
            for (method, present) in methods {
                if present {
                    ops.insert((method.to_string(), normalize_params(&path)));
                }
            }
        }

        ops
    }

    /// A route that is registered but missing from its feature `ApiDoc`
    /// silently disappears from /scalar (this happened to the OAuth routes).
    #[test]
    fn test_every_registered_route_is_in_the_openapi_spec() {
        let documented = documented_operations();
        let missing: Vec<String> = registered_routes()
            .into_iter()
            .filter(|r| !documented.contains(&(r.method.clone(), r.normalized_path())))
            .map(|r| format!("{} {} ({})", r.method, r.path, r.feature))
            .collect();

        assert!(
            missing.is_empty(),
            "routes missing from the OpenAPI spec — add the handler to paths(...) in \
             <feature>/mod.rs and check its #[utoipa::path] method/path:\n  {}",
            missing.join("\n  ")
        );
    }

    /// The reverse: the spec must not advertise routes that do not exist
    /// (stale `#[utoipa::path]` after a rename, wrong method or path).
    #[test]
    fn test_every_openapi_operation_is_a_registered_route() {
        let registered: BTreeSet<(String, String)> = registered_routes()
            .into_iter()
            .map(|r| (r.method.clone(), r.normalized_path()))
            .collect();
        let stale: Vec<String> = documented_operations()
            .into_iter()
            .filter(|op| !registered.contains(op))
            .map(|(method, path)| format!("{method} {path}"))
            .collect();

        assert!(
            stale.is_empty(),
            "OpenAPI operations with no registered route:\n  {}",
            stale.join("\n  ")
        );
    }
}
