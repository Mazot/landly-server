use super::{
    requests::{SignInRequest, SignUpRequest, AddLanguagesRequest, DeleteLanguageRequest}
};
use crate::{app::drivers::middlewares::state::AppState, error::AppError};
use actix_web::{web::{Data, Json, Path}, HttpResponse};
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
    form: Json<SignInRequest>
) -> Result<HttpResponse, AppError> {
    state.
        di_container
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
    form: Json<SignUpRequest>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .signup(form.username.to_owned(), form.email.to_owned(), form.password.to_owned())
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
    form: Json<AddLanguagesRequest>
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
    form: Json<DeleteLanguageRequest>
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
    user_id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .fetch_languages(user_id.into_inner())
}
