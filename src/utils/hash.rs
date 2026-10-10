use crate::error::AppError;
pub use bcrypt::verify;
use bcrypt::{BcryptResult, DEFAULT_COST, hash};
use serde_json::json;

/// Hashes a plaintext password with bcrypt at the default cost.
/// Verification goes through the re-exported `verify`.
pub fn hash_password(naive_password: &str) -> BcryptResult<String> {
    hash(naive_password, DEFAULT_COST)
}

/// Succeeds only when `password` matches `password_hash`.
///
/// `bcrypt::verify` reports a wrong password as `Ok(false)`, not as an error,
/// so its result must never be consumed with a bare `?` — that would accept
/// any password. Every login path goes through this helper instead.
pub fn ensure_password_matches(password: &str, password_hash: &str) -> Result<(), AppError> {
    if verify(password, password_hash)? {
        return Ok(());
    }

    Err(invalid_credentials())
}

/// The single 401 returned for every failed login, so callers cannot tell
/// which part of the credentials was wrong.
pub fn invalid_credentials() -> AppError {
    AppError::Unauthorized(json!({ "error": "Invalid email or password" }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_password_matches_accepts_correct_password() {
        let hashed = hash_password("correct_password").unwrap();

        assert!(ensure_password_matches("correct_password", &hashed).is_ok());
    }

    /// Regression: signin used `verify(..)?` and ignored the returned bool,
    /// so any password produced a valid JWT.
    #[test]
    fn test_ensure_password_matches_rejects_wrong_password() {
        let hashed = hash_password("correct_password").unwrap();

        match ensure_password_matches("wrong_password", &hashed) {
            Err(AppError::Unauthorized(_)) => (),
            other => panic!("expected Unauthorized, got {:?}", other),
        }
    }

    #[test]
    fn test_hash_password_success() {
        let password = "test_password_123";
        let result = hash_password(password);
        assert!(result.is_ok());

        let hashed = result.unwrap();
        assert!(!hashed.is_empty());
        assert_ne!(hashed, password);
    }

    #[test]
    fn test_hash_password_different_hashes() {
        let password = "test_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes due to salt
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_success() {
        let password = "my_secure_password";
        let hashed = hash_password(password).unwrap();

        let result = verify(password, &hashed);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_wrong_password() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hashed = hash_password(password).unwrap();

        let result = verify(wrong_password, &hashed);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_hash_empty_password() {
        let password = "";
        let result = hash_password(password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_long_password() {
        let password = "a".repeat(100);
        let result = hash_password(&password);
        assert!(result.is_ok());

        let hashed = result.unwrap();
        assert!(verify(&password, &hashed).unwrap());
    }
}
