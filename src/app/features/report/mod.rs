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
    paths(controllers::create_report),
    components(schemas(requests::CreateReportRequest, presenters::ReportContent)),
    tags(
        (name = "Report", description = "User reports on orgs/people/conversations for moderation")
    )
)]
pub struct ApiDoc;
