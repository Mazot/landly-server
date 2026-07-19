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
        controllers::create_saved,
        controllers::delete_saved,
        controllers::list_saved,
        controllers::counts_saved,
    ),
    components(schemas(
        requests::CreateSavedRequest,
        requests::ListSavedQueryRequest,
        presenters::SavedItemContent,
        presenters::MultipleSavedItemsResponse,
        presenters::SavedCountsContent,
    )),
    tags(
        (name = "Saved", description = "Private bookmarks (org/person/country/corridor) with notes")
    )
)]
pub struct ApiDoc;
