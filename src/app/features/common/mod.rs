pub mod config;
pub mod controllers;
pub mod presenters;
pub mod repositories;
pub mod usecases;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(
        controllers::fetch_all_countries,
        controllers::fetch_country_detail,
        controllers::fetch_all_languages,
        controllers::fetch_all_organisation_types,
        controllers::create_organisation_type,
    ),
    components(schemas(
        presenters::CountryContent,
        presenters::CountryDetailContent,
        presenters::CountryPlacesByType,
        presenters::OrganisationTypeContent,
        presenters::LanguageContent,
        controllers::CountriesListQueryParams,
        controllers::LanguagesListQueryParams,
        controllers::CreateOrganisationTypeRequest,
    )),
    tags((name = "Common", description = "Common reference data: countries, languages, organisation types."))
)]
pub struct ApiDoc;
