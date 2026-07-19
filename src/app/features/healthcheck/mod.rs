pub mod controllers;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(controllers::index),
    tags((name = "Healthcheck", description = "Healthcheck related endpoints"))
)]
pub struct ApiDoc;
