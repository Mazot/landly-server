use crate::{constants::env_key, error::AppError};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// https://auth0.com/docs/secure/tokens/json-web-tokens/json-web-token-claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Authenticated user ID; the auth middleware puts it into the request extensions.
    pub sub: Uuid,
    /// Expiry (Unix seconds): issue time + `JWT_EXPIRATION`.
    exp: usize,
    /// Issue time (Unix seconds).
    iat: usize,
}

/// Shortest accepted `JWT_SECRET`, in bytes (HS256 wants at least the 256-bit
/// hash size).
pub const MIN_JWT_SECRET_BYTES: usize = 32;

/// Fewest distinct characters a secret must contain; rules out `aaaa…`,
/// `0101…` and repeated short phrases.
const MIN_DISTINCT_CHARS: usize = 10;

/// Fragments of the placeholder values that shipped in `.env.example`, the
/// README and tutorials. A secret containing one was copied, not generated.
const PLACEHOLDER_FRAGMENTS: [&str; 9] = [
    "changeme",
    "change-me",
    "change_me",
    "your-",
    "your_",
    "example",
    "placeholder",
    "super-secret",
    "secret-jwt",
];

/// Checks that `JWT_SECRET` can safely sign tokens: anyone who guesses it can
/// mint a token for any user, including an admin (audit H2). Called once at
/// startup so a weak secret stops the server instead of running exposed.
pub fn validate_jwt_secret(secret: &str) -> Result<(), String> {
    let how = "generate one with `openssl rand -hex 32`";
    if secret.len() < MIN_JWT_SECRET_BYTES {
        return Err(format!(
            "JWT_SECRET is too short ({} bytes, need at least {MIN_JWT_SECRET_BYTES}); {how}",
            secret.len()
        ));
    }
    let lowered = secret.to_lowercase();
    if PLACEHOLDER_FRAGMENTS.iter().any(|p| lowered.contains(p)) {
        return Err(format!(
            "JWT_SECRET looks like a placeholder from the docs; {how}"
        ));
    }
    let distinct = secret
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    if distinct < MIN_DISTINCT_CHARS {
        return Err(format!(
            "JWT_SECRET is too repetitive ({distinct} distinct characters, need {MIN_DISTINCT_CHARS}); {how}"
        ));
    }
    Ok(())
}

/// Validates the `JWT_SECRET` environment variable (see [`validate_jwt_secret`]).
pub fn validate_jwt_secret_env() -> Result<(), String> {
    match std::env::var(env_key::JWT_SECRET) {
        Ok(secret) => validate_jwt_secret(&secret),
        Err(_) => Err("JWT_SECRET is not set; generate one with `openssl rand -hex 32`".into()),
    }
}

fn get_jwt_secret() -> String {
    std::env::var(env_key::JWT_SECRET).expect("JWT_SECRET must be set")
}

fn get_jwt_expiration_secs() -> usize {
    std::env::var(env_key::JWT_EXPIRATION)
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(3600)
}

/// Validates a JWT signature + expiry and returns its claims.
/// The auth middleware uses `claims.sub` as the authenticated user id.
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

/// Issues a signed JWT for the user, expiring after `JWT_EXPIRATION` seconds
/// (default 3600).
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
    use serial_test::serial;

    #[test]
    fn test_validate_jwt_secret_accepts_generated_secrets() {
        // `openssl rand -hex 32` and `openssl rand -base64 48` shapes.
        assert!(
            validate_jwt_secret("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
                .is_ok()
        );
        assert!(
            validate_jwt_secret("Zx3/9qLm+Vt0aPb7KdWc2YhNfRu8EoJi5GsTy1XnMwQ4eAvB6CkDlHgUjOpIrSzF")
                .is_ok()
        );
    }

    #[test]
    fn test_validate_jwt_secret_rejects_weak_secrets() {
        for weak in [
            "",
            "0987654321",
            "test_secret",
            "your-super-secret-jwt-key",
            // long enough, but copied from docs / not random
            "your-super-secret-jwt-key-change-me-please",
            "0123012301230123012301230123012301230123",
            "ChangeMeChangeMeChangeMeChangeMe1234",
        ] {
            assert!(
                validate_jwt_secret(weak).is_err(),
                "{weak:?} must be rejected"
            );
        }
        assert!(validate_jwt_secret(&"k".repeat(64)).is_err());
        assert!(validate_jwt_secret(&"k".repeat(MIN_JWT_SECRET_BYTES - 1)).is_err());
    }

    #[test]
    #[serial(jwt_env)]
    fn test_validate_jwt_secret_env_requires_the_variable() {
        // SAFETY: serialized by #[serial(jwt_env)].
        unsafe { std::env::remove_var("JWT_SECRET") };
        assert!(validate_jwt_secret_env().is_err());
        unsafe { std::env::set_var("JWT_SECRET", "short") };
        assert!(validate_jwt_secret_env().is_err());
        unsafe {
            std::env::set_var(
                "JWT_SECRET",
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822c",
            )
        };
        assert!(validate_jwt_secret_env().is_ok());
    }

    fn setup_test_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_for_testing");
            std::env::set_var("JWT_EXPIRATION", "3600");
        }
    }

    #[test]
    #[serial(jwt_env)]
    fn test_generate_token_success() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let result = generate_token(user_id);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    #[serial(jwt_env)]
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
    #[serial(jwt_env)]
    fn test_decode_invalid_token() {
        setup_test_env();
        let invalid_token = "invalid.token.here";

        let result = decode_token(invalid_token);
        assert!(result.is_err());
    }

    #[test]
    #[serial(jwt_env)]
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
    #[serial(jwt_env)]
    fn test_get_jwt_expiration_default() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::remove_var("JWT_EXPIRATION");
        }

        let exp = get_jwt_expiration_secs();
        assert_eq!(exp, 3600);
    }

    #[test]
    #[serial(jwt_env)]
    fn test_get_jwt_expiration_custom() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::set_var("JWT_EXPIRATION", "7200");
        }

        let exp = get_jwt_expiration_secs();
        assert_eq!(exp, 7200);
    }

    #[test]
    #[serial(jwt_env)]
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
    #[serial(jwt_env)]
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
