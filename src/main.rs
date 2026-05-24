#![allow(dead_code)]
mod app;
mod constants;
pub mod data;
mod error;
pub mod utils;

use crate::app::drivers::middlewares::{auth::Authentication, cors::cors};
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Landly Web API",
        description = "Landly OpenAPI Specification."
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.orsoft.xyz", description = "Production server"),
    ),
    paths(
        app::features::healthcheck::controllers::index,
        app::features::common::controllers::fetch_all_countries,
        app::features::common::controllers::fetch_all_organisation_types,
        app::features::common::controllers::create_organisation_type,
        app::features::organisation::controllers::list_organisations,
        app::features::organisation::controllers::fetch_organisation,
        app::features::organisation::controllers::create_organisation,
        app::features::organisation::controllers::delete_organisation,
        app::features::organisation::controllers::update_organisation,
        app::features::country_connection::controllers::list,
        app::features::country_connection::controllers::fetch,
        app::features::country_connection::controllers::create,
        app::features::country_connection::controllers::delete,
        app::features::country_connection::controllers::update,
        app::features::user::controllers::signin,
        app::features::user::controllers::signup,
        app::features::user::controllers::add_languages,
        app::features::user::controllers::delete_language,
        app::features::user::controllers::fetch_languages,
        app::features::images::controllers::upload_image,
        app::features::images::controllers::delete_image,
        app::features::images::controllers::list_images,
        app::features::images::controllers::fetch_image,
        app::features::images::controllers::set_primary_image,
    ),
    components(
        schemas(
            app::features::common::presenters::CountryContent,
            app::features::common::presenters::OrganisationTypeContent,
            app::features::common::controllers::CountriesListQueryParams,
            app::features::common::controllers::CreateOrganisationTypeRequest,
            app::features::organisation::requests::OrganisationsListQueryRequest,
            app::features::organisation::requests::CreateOrganisationRequest,
            app::features::organisation::requests::UpdateOrganisationRequest,
            app::features::organisation::presenters::OrganisationContent,
            app::features::organisation::presenters::MultipleOrganisationsResponse,
            app::features::country_connection::requests::CreateCountryConnectionRequest,
            app::features::country_connection::requests::UpdateCountryConnectionRequest,
            app::features::country_connection::requests::CountryConnectionsListQueryParams,
            app::features::country_connection::presenters::CountryConnectionContent,
            app::features::country_connection::presenters::MultipleCountryConnectionsResponse,
            app::features::user::requests::SignInRequest,
            app::features::user::requests::SignUpRequest,
            app::features::user::requests::AddLanguagesRequest,
            app::features::user::requests::DeleteLanguageRequest,
            app::features::user::presenters::AuthUserContent,
            app::features::user::presenters::UserLanguagesContent,
            app::features::images::presenters::ImageContent,
            app::features::images::presenters::MultipleImagesResponse,
            app::features::images::requests::ImagesListQueryParams,
        )
    ),
    tags(
        (name = "Healthcheck", description = "Healthcheck related endpoints"),
        (name = "Common", description = "Common endpoints like countries, etc."),
        (name = "Organisation", description = "Organisation related endpoints"),
        (name = "CountryConnection", description = "CountryConnection related endpoints"),
        (name = "User", description = "User related endpoints"),
        (name = "Images", description = "Image upload and management endpoints")
    )
)]
pub struct ApiDoc;

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
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .service(web::redirect("/swagger-ui", "/swagger-ui/"))
            .service(
                web::scope("/api")
                    .service(web::scope("/healthcheck").route(
                        "",
                        web::get().to(app::features::healthcheck::controllers::index),
                    ))
                    .configure(app::features::common::config::configure_services)
                    .configure(app::features::user::config::configure_services)
                    .configure(organisation_configure_services)
                    .configure(country_connection_configure_services)
                    .configure(images_configure_services),
            )
    })
    .bind(constants::BIND)?
    .run()
    .await
}
