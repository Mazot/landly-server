pub mod config;
pub mod controllers;
pub mod entities;
pub mod presenters;
pub mod repositories;
pub mod requests;
pub mod usecases;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(
        controllers::create_review,
        controllers::list_reviews,
        controllers::delete_review,
    ),
    components(schemas(
        requests::CreateReviewRequest,
        requests::ListReviewsQueryRequest,
        presenters::ReviewContent,
        presenters::MultipleReviewsResponse,
    )),
    tags(
        (name = "Review", description = "Reviews for organisations and people with live aggregates")
    )
)]
pub struct ApiDoc;
