use super::entities::{User, UserToLanguage};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait UserPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, user: User, token: String) -> HttpResponse;
    fn to_lang_vec_json(&self, languages: Vec<UserToLanguage>) -> HttpResponse;
    fn to_auth_middleware(&self, maybe_user: Result<User, AppError>) -> Result<User, &str>;
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserLanguagesContent {
    pub user_id: Uuid,
    pub language_ids: Vec<Uuid>,
}

impl From<Vec<UserToLanguage>> for UserLanguagesContent {
    fn from(value: Vec<UserToLanguage>) -> Self {
        let user_id = value.first().map(|v| v.id).unwrap_or_default();
        let language_ids: Vec<Uuid> = value.into_iter().map(|v| v.language_id).collect();

        Self {
            user_id,
            language_ids,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserContent {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub token: String,
}

impl From<(User, String)> for AuthUserContent {
    fn from(user_and_token: (User, String)) -> Self {
        let (user, token) = user_and_token;
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            token,
        }
    }
}

#[derive(Clone)]
pub struct UserPresenterImpl {}
impl UserPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl UserPresenter for UserPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, user: User, token: String) -> HttpResponse {
        let response_content = AuthUserContent::from((user, token));

        HttpResponse::Ok().json(response_content)
    }

    fn to_auth_middleware(&self, maybe_user: Result<User, AppError>) -> Result<User, &str> {
        maybe_user.map_err(|_err| "Cannot find auth user")
    }

    fn to_lang_vec_json(&self, languages: Vec<UserToLanguage>) -> HttpResponse {
        let response_content: UserLanguagesContent = languages.into();

        HttpResponse::Ok().json(response_content)
    }
}
