use super::requests::{
    AddLanguagesRequest, DeleteLanguageRequest, OAuthCallbackParams, SignInRequest, SignUpRequest,
};
use crate::{app::drivers::middlewares::state::AppState, error::AppError};
use actix_web::{
    HttpResponse,
    web::{Data, Json, Path, Query},
};
use uuid::Uuid;

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
    state.di_container.user_usecase.signup(
        form.username.to_owned(),
        form.email.to_owned(),
        form.password.to_owned(),
    )
}

#[utoipa::path(
    post,
    path = "/user/languages",
    context_path = "/api",
    request_body = AddLanguagesRequest,
    responses(
        (status = 200, description = "Languages added successfully", body = super::presenters::UserLanguagesContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn add_languages(
    state: Data<AppState>,
    form: Json<AddLanguagesRequest>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .add_languages(form.user_id, form.languages_ids.clone())
}

#[utoipa::path(
    delete,
    path = "/user/languages",
    context_path = "/api",
    request_body = DeleteLanguageRequest,
    responses(
        (status = 200, description = "Language deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn delete_language(
    state: Data<AppState>,
    form: Json<DeleteLanguageRequest>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .delete_language(form.user_id, form.language_id)
}

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

    let response = state
        .di_container
        .user_usecase
        .oauth_google_upsert(email, user_info.sub)?;

    Ok(response)
}
