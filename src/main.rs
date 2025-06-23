mod constants;
mod app;
mod error; 
pub mod data;
pub mod utils;

use crate::app::drivers::middlewares::{
    cors::cors,
};
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
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
        (url = "https://api.orsoft.com", description = "Production server"),
    ),
    paths(
        app::features::healthcheck::controllers::index,
        app::features::common::controllers::fetch_all_countries,
        app::features::organisation::controllers::list,
        app::features::organisation::controllers::fetch,
        app::features::organisation::controllers::create,
        app::features::organisation::controllers::delete,
        app::features::organisation::controllers::update,
    ),
    components(
        schemas(
            app::features::common::presenters::CountryContent,
            app::features::common::presenters::OrganisationTypeContent,
            app::features::organisation::requests::OrganisationsListQueryRequest,
            app::features::organisation::requests::CreateOrganisationRequest,
            app::features::organisation::requests::UpdateOrganisationRequest,
            app::features::organisation::presenters::OrganisationContent,
            app::features::organisation::presenters::MultipleOrganisationsResponse,
        )
    ),
    tags(
        (name = "Healthcheck", description = "Healthcheck related endpoints"),
        (name = "Common", description = "Common endpoints like countries, etc."),
        (name = "Organisation", description = "Organisation related endpoints")
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
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(Logger::default())
            .wrap(cors())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
            .service(
                web::redirect("/swagger-ui", "/swagger-ui/")
            )
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/healthcheck")
                            .route("", web::get().to(app::features::healthcheck::controllers::index))
                    )
                    .configure(app::features::common::config::configure_services)
                    .configure(app::features::organisation::config::configure_services)
            )
    })
    .bind(constants::BIND)?
    .run()
    .await
}
