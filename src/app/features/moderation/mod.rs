pub mod config;
pub mod controllers;
pub mod presenters;
pub mod repositories;
pub mod requests;
pub mod usecases;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(
        controllers::fetch_queue,
        controllers::approve,
        controllers::request_changes,
        controllers::reject,
    ),
    components(schemas(
        requests::ModerationQueueQueryRequest,
        requests::ModerationDecisionRequest,
        presenters::ModerationQueueResponse,
        presenters::ModerationQueueItemContent,
        presenters::ModerationEventContent,
    )),
    tags(
        (name = "Moderation", description = "Moderator-only review queue for pending orgs and people")
    )
)]
pub struct ApiDoc;
