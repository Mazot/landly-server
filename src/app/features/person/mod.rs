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
        controllers::create_person,
        controllers::list_people,
        controllers::fetch_person,
        controllers::vouch_person,
        controllers::claim_preview,
        controllers::claim_confirm,
        controllers::claim_decline,
    ),
    components(schemas(
        requests::CreatePersonRequest,
        requests::ListPeopleQueryRequest,
        requests::VouchPersonRequest,
        requests::ClaimConfirmRequest,
        presenters::PersonContent,
        presenters::PersonCreatedContent,
        presenters::MultiplePeopleResponse,
        presenters::ClaimPreviewContent,
    )),
    tags(
        (name = "Person", description = "Recommended people / helpers with a claim-and-verify flow")
    )
)]
pub struct ApiDoc;
