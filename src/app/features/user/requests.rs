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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_in_request_serialization() {
        let request = SignInRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("password123"));
    }

    #[test]
    fn test_sign_in_request_deserialization() {
        let json = r#"{"email":"user@test.com","password":"secret"}"#;
        
        let request: SignInRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.email, "user@test.com");
        assert_eq!(request.password, "secret");
    }

    #[test]
    fn test_sign_up_request_serialization() {
        let request = SignUpRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "newpass123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("newuser"));
        assert!(json.contains("new@example.com"));
    }

    #[test]
    fn test_sign_up_request_deserialization() {
        let json = r#"{"username":"testuser","email":"test@example.com","password":"testpass"}"#;
        
        let request: SignUpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "testuser");
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "testpass");
    }

    #[test]
    fn test_add_languages_request_single_language() {
        let user_id = Uuid::new_v4();
        let lang_id = Uuid::new_v4();
        
        let request = AddLanguagesRequest {
            user_id,
            languages_ids: vec![lang_id],
        };

        assert_eq!(request.user_id, user_id);
        assert_eq!(request.languages_ids.len(), 1);
        assert_eq!(request.languages_ids[0], lang_id);
    }

    #[test]
    fn test_add_languages_request_multiple_languages() {
        let user_id = Uuid::new_v4();
        let lang1 = Uuid::new_v4();
        let lang2 = Uuid::new_v4();
        let lang3 = Uuid::new_v4();
        
        let request = AddLanguagesRequest {
            user_id,
            languages_ids: vec![lang1, lang2, lang3],
        };

        assert_eq!(request.languages_ids.len(), 3);
        assert!(request.languages_ids.contains(&lang1));
        assert!(request.languages_ids.contains(&lang2));
        assert!(request.languages_ids.contains(&lang3));
    }

    #[test]
    fn test_add_languages_request_empty_list() {
        let user_id = Uuid::new_v4();
        
        let request = AddLanguagesRequest {
            user_id,
            languages_ids: vec![],
        };

        assert_eq!(request.languages_ids.len(), 0);
    }

    #[test]
    fn test_delete_language_request() {
        let user_id = Uuid::new_v4();
        let language_id = Uuid::new_v4();
        
        let request = DeleteLanguageRequest {
            user_id,
            language_id,
        };

        assert_eq!(request.user_id, user_id);
        assert_eq!(request.language_id, language_id);
    }

    #[test]
    fn test_delete_language_request_serialization() {
        let user_id = Uuid::new_v4();
        let language_id = Uuid::new_v4();
        
        let request = DeleteLanguageRequest {
            user_id,
            language_id,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(&user_id.to_string()));
        assert!(json.contains(&language_id.to_string()));
    }

    #[test]
    fn test_oauth_callback_params_with_scope() {
        let json = r#"{"code":"auth_code_123","state":"state_xyz","scope":"read write"}"#;
        
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.code, "auth_code_123");
        assert_eq!(params.state, "state_xyz");
        assert_eq!(params.scope, Some("read write".to_string()));
    }

    #[test]
    fn test_oauth_callback_params_without_scope() {
        let json = r#"{"code":"auth_code_456","state":"state_abc"}"#;
        
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.code, "auth_code_456");
        assert_eq!(params.state, "state_abc");
        assert!(params.scope.is_none());
    }

    #[test]
    fn test_sign_up_request_with_special_characters() {
        let request = SignUpRequest {
            username: "user-name_123".to_string(),
            email: "test+tag@example.co.uk".to_string(),
            password: "P@ssw0rd!#$".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SignUpRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.username, request.username);
        assert_eq!(deserialized.email, request.email);
        assert_eq!(deserialized.password, request.password);
    }
}
