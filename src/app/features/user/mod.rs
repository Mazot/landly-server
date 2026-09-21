pub mod config;
pub mod controllers;
pub mod entities;
pub mod oauth;
pub mod presenters;
pub mod repositories;
pub mod requests;
pub mod usecases;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(
        controllers::signin,
        controllers::signup,
        controllers::fetch_me,
        controllers::update_me,
        controllers::update_notification_settings,
        controllers::add_languages,
        controllers::delete_language,
        controllers::fetch_languages,
    ),
    components(schemas(
        requests::SignInRequest,
        requests::SignUpRequest,
        requests::UpdateProfileRequest,
        requests::UpdateNotificationSettingsRequest,
        requests::AddLanguagesRequest,
        requests::DeleteLanguageRequest,
        presenters::AuthUserContent,
        presenters::UserProfileContent,
        presenters::UserProfileStatsContent,
        presenters::UserLanguagesContent,
    )),
    tags((name = "User", description = "User related endpoints"))
)]
pub struct ApiDoc;
