mod constants;
mod app;
mod error;
mod data;

use app::drivers::middlewares::{
    cors::cors,
};
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    println!("start server");
    // std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(cors())
    })
    .bind(constants::BIND)?
    .run()
    .await
}
