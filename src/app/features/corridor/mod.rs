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
        controllers::create_corridor,
        controllers::list_corridors,
        controllers::set_default_corridor,
        controllers::delete_corridor,
        controllers::fetch_corridor_stats,
    ),
    components(schemas(
        requests::CreateCorridorRequest,
        presenters::CorridorContent,
        presenters::MultipleCorridorsResponse,
        presenters::CorridorStatsContent,
        presenters::CorridorTypeCount,
    )),
    tags((name = "Corridor", description = "User corridor (from → to country) endpoints"))
)]
pub struct ApiDoc;
