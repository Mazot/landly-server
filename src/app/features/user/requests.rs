use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct SignUpRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct AddLanguagesRequest {
    pub user_id: Uuid,
    pub languages_ids: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct DeleteLanguageRequest {
    pub user_id: Uuid,
    pub language_id: Uuid,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: String,
    pub scope: Option<String>,
}
