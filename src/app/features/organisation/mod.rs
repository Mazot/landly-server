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
        controllers::list_organisations,
        controllers::search_organisations,
        controllers::fetch_organisation,
        controllers::create_organisation,
        controllers::delete_organisation,
        controllers::update_organisation,
        controllers::visit_organisation,
        controllers::checkin_organisation,
    ),
    components(schemas(
        requests::OrganisationsListQueryRequest,
        requests::SearchOrganisationsQueryRequest,
        requests::CreateOrganisationRequest,
        requests::UpdateOrganisationRequest,
        presenters::OrganisationContent,
        presenters::MultipleOrganisationsResponse,
        presenters::OrganisationVisitsContent,
        presenters::CommunitySignalsContent,
        requests::CheckinRequest,
    )),
    tags((name = "Organisation", description = "Organisation related endpoints"))
)]
pub struct ApiDoc;
