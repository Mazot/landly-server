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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("start server");
    // std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    // TODO: Tmp solution
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
                web::resource("/test")
                    .route(web::post().to(app::features::organisation::controllers::create))
            )
            .service(
                web::scope("/healthcheck")
                    .route("", web::get().to(app::features::healthcheck::controllers::index))
            )
            .service(
                web::scope("/common")
                    .route("/countries", web::get().to(app::features::common::controllers::fetch_all_countries))
            )
    })
    .bind(constants::BIND)?
    .run()
    .await
}
