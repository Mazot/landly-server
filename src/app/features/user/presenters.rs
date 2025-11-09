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
        let user_id = value.first().map(|v| v.user_id).unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$2b$12$test_hash".to_string(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    fn create_test_user_to_language(user_id: Uuid, language_id: Uuid) -> UserToLanguage {
        UserToLanguage {
            user_id,
            language_id,
        }
    }

    #[test]
    fn test_auth_user_content_from_user_and_token() {
        let user = create_test_user();
        let user_id = user.id;
        let token = "test_token_123".to_string();
        
        let content = AuthUserContent::from((user, token.clone()));
        
        assert_eq!(content.id, user_id);
        assert_eq!(content.username, "testuser");
        assert_eq!(content.email, "test@example.com");
        assert_eq!(content.token, token);
    }

    #[test]
    fn test_user_languages_content_from_vec() {
        let user_id = Uuid::new_v4();
        let lang1 = Uuid::new_v4();
        let lang2 = Uuid::new_v4();
        let lang3 = Uuid::new_v4();
        
        let languages = vec![
            create_test_user_to_language(user_id, lang1),
            create_test_user_to_language(user_id, lang2),
            create_test_user_to_language(user_id, lang3),
        ];
        
        let content = UserLanguagesContent::from(languages);
        
        assert_eq!(content.user_id, user_id);
        assert_eq!(content.language_ids.len(), 3);
        assert!(content.language_ids.contains(&lang1));
        assert!(content.language_ids.contains(&lang2));
        assert!(content.language_ids.contains(&lang3));
    }

    #[test]
    fn test_user_languages_content_from_empty_vec() {
        let languages: Vec<UserToLanguage> = vec![];
        
        let content = UserLanguagesContent::from(languages);
        
        assert_eq!(content.user_id, Uuid::default());
        assert_eq!(content.language_ids.len(), 0);
    }

    #[test]
    fn test_user_languages_content_single_language() {
        let user_id = Uuid::new_v4();
        let lang_id = Uuid::new_v4();
        
        let languages = vec![create_test_user_to_language(user_id, lang_id)];
        
        let content = UserLanguagesContent::from(languages);
        
        assert_eq!(content.user_id, user_id);
        assert_eq!(content.language_ids.len(), 1);
        assert_eq!(content.language_ids[0], lang_id);
    }

    #[test]
    fn test_user_presenter_new() {
        let presenter = UserPresenterImpl::new();
        assert!(std::mem::size_of_val(&presenter) == 0);
    }

    #[test]
    fn test_user_presenter_to_http_res() {
        let presenter = UserPresenterImpl::new();
        let response = presenter.to_http_res();
        
        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_user_presenter_to_single_json() {
        let presenter = UserPresenterImpl::new();
        let user = create_test_user();
        let token = "test_token".to_string();
        
        let response = presenter.to_single_json(user, token);
        
        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_user_presenter_to_lang_vec_json() {
        let presenter = UserPresenterImpl::new();
        let user_id = Uuid::new_v4();
        let languages = vec![
            create_test_user_to_language(user_id, Uuid::new_v4()),
            create_test_user_to_language(user_id, Uuid::new_v4()),
        ];
        
        let response = presenter.to_lang_vec_json(languages);
        
        assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_user_presenter_to_auth_middleware_success() {
        let presenter = UserPresenterImpl::new();
        let user = create_test_user();
        let user_id = user.id;
        
        let result = presenter.to_auth_middleware(Ok(user));
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, user_id);
    }

    #[test]
    fn test_user_presenter_to_auth_middleware_failure() {
        let presenter = UserPresenterImpl::new();
        let error = AppError::Unauthorized(serde_json::json!({"error": "test"}));
        
        let result = presenter.to_auth_middleware(Err(error));
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot find auth user");
    }

    #[test]
    fn test_user_presenter_clone() {
        let presenter = UserPresenterImpl::new();
        let _cloned = presenter.clone();
        
        assert!(true);
    }

    #[test]
    fn test_auth_user_content_serialization() {
        let user = create_test_user();
        let token = "test_token".to_string();
        let content = AuthUserContent::from((user, token));
        
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("test_token"));
    }

    #[test]
    fn test_user_languages_content_serialization() {
        let user_id = Uuid::new_v4();
        let content = UserLanguagesContent {
            user_id,
            language_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("userId"));
        assert!(json.contains("languageIds"));
    }
}
