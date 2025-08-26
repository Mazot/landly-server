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
struct JwtClaims {
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
