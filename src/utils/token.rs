use crate::{
    constants::env_key,
    error::AppError,
};
use jsonwebtoken::{EncodingKey, Header};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// https://auth0.com/docs/secure/tokens/json-web-tokens/json-web-token-claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,
    exp: usize,
    iat: usize,
}

fn get_jwt_secret() -> String {
    std::env::var(env_key::JWT_SECRET)
        .expect("JWT_SECRET must be set")
}

fn get_jwt_expiration_secs() -> usize {
    std::env::var(env_key::JWT_EXPIRATION)
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(3600)
}

pub fn decode_token(token: &str) -> Result<JwtClaims, AppError> {
    let secret = get_jwt_secret();
    let validation = jsonwebtoken::Validation::default();

    let token_data = jsonwebtoken::decode::<JwtClaims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn generate_token(user_id: Uuid) -> Result<String, AppError> {
    let now = Utc::now();
    let exp_secs = get_jwt_expiration_secs();
    let secret = get_jwt_secret();

    let claims = JwtClaims {
        sub: user_id.to_owned(),
        exp: (now.timestamp() + exp_secs as i64) as usize,
        iat: now.timestamp() as usize,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::InternalServerError)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_for_testing");
            std::env::set_var("JWT_EXPIRATION", "3600");
        }
    }

    #[test]
    fn test_generate_token_success() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let result = generate_token(user_id);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_decode_token_success() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let token = generate_token(user_id).unwrap();

        let result = decode_token(&token);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_decode_invalid_token() {
        setup_test_env();
        let invalid_token = "invalid.token.here";

        let result = decode_token(invalid_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_claims_structure() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let token = generate_token(user_id).unwrap();
        let claims = decode_token(&token).unwrap();

        assert_eq!(claims.sub, user_id);
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 3600);
    }

    #[test]
    fn test_get_jwt_expiration_default() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::remove_var("JWT_EXPIRATION");
        }

        let exp = get_jwt_expiration_secs();
        assert_eq!(exp, 3600);
    }

    #[test]
    fn test_get_jwt_expiration_custom() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::set_var("JWT_EXPIRATION", "7200");
        }

        let exp = get_jwt_expiration_secs();
        assert_eq!(exp, 7200);
    }

    #[test]
    fn test_different_tokens_for_different_users() {
        setup_test_env();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        let token1 = generate_token(user1).unwrap();
        let token2 = generate_token(user2).unwrap();

        assert_ne!(token1, token2);

        let claims1 = decode_token(&token1).unwrap();
        let claims2 = decode_token(&token2).unwrap();

        assert_eq!(claims1.sub, user1);
        assert_eq!(claims2.sub, user2);
    }

    #[test]
    fn test_token_with_wrong_secret() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let token = generate_token(user_id).unwrap();

        // Change the secret
        unsafe {
            std::env::set_var("JWT_SECRET", "different_secret");
        }

        let result = decode_token(&token);
        assert!(result.is_err());
    }
}
