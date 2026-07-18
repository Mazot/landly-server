use super::requests::{
    AddLanguagesRequest, DeleteLanguageRequest, OAuthCallbackParams, SignInRequest, SignUpRequest,
    UpdateNotificationSettingsRequest, UpdateProfileRequest,
};
use super::usecases::{SignUpUsecaseInput, UpdateProfileUsecaseInput};
use crate::{app::drivers::middlewares::state::AppState, constants::env_key, error::AppError};
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    post,
    path = "/user/signin",
    context_path = "/api",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "User signed in successfully", body = super::presenters::AuthUserContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn signin(
    state: Data<AppState>,
    form: Json<SignInRequest>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .signin(form.email.to_owned(), form.password.to_owned())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// signup v2 — optional profile fields + default corridor in one transaction.
#[utoipa::path(
    post,
    path = "/user/signup",
    context_path = "/api",
    request_body = SignUpRequest,
    responses(
        (status = 200, description = "User signed up successfully", body = super::presenters::AuthUserContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn signup(
    state: Data<AppState>,
    form: Json<SignUpRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.user_usecase.signup(SignUpUsecaseInput {
        username: form.username.to_owned(),
        email: form.email.to_owned(),
        password: form.password.to_owned(),
        name: form.name.to_owned(),
        locale: form.locale.to_owned(),
        here_as: form.here_as.to_owned(),
        home_country_id: form.home_country_id,
        avatar_color: form.avatar_color.to_owned(),
        corridor_from_country_id: form.corridor_from_country_id,
        corridor_to_country_id: form.corridor_to_country_id,
    })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/user/me",
    context_path = "/api",
    responses(
        (status = 200, description = "Authenticated user profile", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn fetch_me(req: HttpRequest, state: Data<AppState>) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.user_usecase.fetch_profile(user_id)
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    put,
    path = "/user/me",
    context_path = "/api",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn update_me(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<UpdateProfileRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.user_usecase.update_profile(
        user_id,
        UpdateProfileUsecaseInput {
            name: form.name.to_owned(),
            bio: form.bio.to_owned(),
            city: form.city.to_owned(),
            home_country_id: form.home_country_id,
            avatar_color: form.avatar_color.to_owned(),
            locale: form.locale.to_owned(),
            here_as: form.here_as.to_owned(),
        },
    )
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    put,
    path = "/user/me/notifications",
    context_path = "/api",
    request_body = UpdateNotificationSettingsRequest,
    responses(
        (status = 200, description = "Notification settings updated", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn update_notification_settings(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<UpdateNotificationSettingsRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .update_notification_settings(user_id, form.notification_settings.to_owned())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// user_id now comes from the JWT instead of the request body.
#[utoipa::path(
    post,
    path = "/user/languages",
    context_path = "/api",
    request_body = AddLanguagesRequest,
    responses(
        (status = 200, description = "Languages added successfully", body = super::presenters::UserLanguagesContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn add_languages(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<AddLanguagesRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .add_languages(user_id, form.languages_ids.clone())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// user_id now comes from the JWT instead of the request body.
#[utoipa::path(
    delete,
    path = "/user/languages",
    context_path = "/api",
    request_body = DeleteLanguageRequest,
    responses(
        (status = 200, description = "Language deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn delete_language(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<DeleteLanguageRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .delete_language(user_id, form.language_id)
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/{user_id}/languages",
    context_path = "/api",
    params(
        ("user_id" = Uuid, Path, description = "User ID to fetch languages for")
    ),
    responses(
        (status = 200, description = "Languages fetched successfully", body = super::presenters::UserLanguagesContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn fetch_languages(
    state: Data<AppState>,
    user_id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .fetch_languages(user_id.into_inner())
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/oauth/google/login",
    context_path = "/api",
    responses(
        (status = 302, description = "Redirect to Google OAuth")
    ),
    tag = "User"
)]
pub async fn oauth_google_login(state: Data<AppState>) -> Result<HttpResponse, AppError> {
    let (url, _state) = state.di_container.oauth_google.auth_url()?;

    Ok(HttpResponse::Found()
        .append_header(("Location", url.to_string()))
        .finish())
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/oauth/google/callback",
    context_path = "/api",
    params(OAuthCallbackParams),
    responses(
        (status = 200, description = "Google OAuth callback handled successfully", body = super::presenters::AuthUserContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn oauth_google_callback(
    state: Data<AppState>,
    query: Query<OAuthCallbackParams>,
) -> Result<HttpResponse, AppError> {
    let user_info = state
        .di_container
        .oauth_google
        .exchange_and_userinfo(query.code.to_owned(), query.state.to_owned())
        .await?;
    let email = user_info
        .email
        .ok_or_else(|| AppError::Unauthorized(serde_json::json!({"message":"no email"})))?;

    let auth = state
        .di_container
        .user_usecase
        .oauth_google_upsert_content(email, user_info.sub)?;

    let frontend_origin = std::env::var(env_key::FRONTEND_ORIGIN)
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

    let redirect_url = format!(
        "{}/auth/google/callback?token={}&id={}&username={}&email={}",
        frontend_origin,
        urlencoding::encode(&auth.token),
        auth.id,
        urlencoding::encode(&auth.username),
        urlencoding::encode(&auth.email),
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .finish())
}
