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
        controllers::list,
        controllers::fetch,
        controllers::create,
        controllers::delete,
        controllers::update,
    ),
    components(schemas(
        requests::CreateCountryConnectionRequest,
        requests::UpdateCountryConnectionRequest,
        requests::CountryConnectionsListQueryParams,
        presenters::CountryConnectionContent,
        presenters::MultipleCountryConnectionsResponse,
    )),
    tags((name = "CountryConnection", description = "CountryConnection related endpoints"))
)]
pub struct ApiDoc;
