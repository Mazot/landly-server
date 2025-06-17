use actix_web::{HttpResponse, Responder};

#[utoipa::path(
    get,
    path = "/healthcheck",
    context_path = "/api",
    responses(
        (status = 200, description = "Healthcheck response", content_type = "application/json" )
    ),
    tag = "Healthcheck"
)]
pub async fn index() -> impl Responder {
    HttpResponse::Ok().json("Healthcheck")
}
