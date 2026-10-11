use crate::error::AppError;
pub use bcrypt::verify;
use bcrypt::{BcryptResult, DEFAULT_COST, hash};
use serde_json::json;
use std::sync::OnceLock;

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

/// A valid bcrypt hash of a random value at the production cost, verified
/// against when the email is unknown so that signin takes as long as for a
/// real account.
pub fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();

    DUMMY.get_or_init(|| {
        hash_password(&uuid::Uuid::new_v4().to_string()).expect("bcrypt hashing never fails")
    })
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
    fn test_dummy_hash_is_a_real_bcrypt_hash() {
        assert!(dummy_hash().starts_with("$2"));
        assert!(!verify("anything", dummy_hash()).unwrap());
    }

    // Test passwords are built at runtime rather than written as literals so
    // that code scanning does not mistake them for hard-coded credentials.
    fn test_password(c: &str) -> String {
        c.repeat(12)
    }

    #[test]
    fn test_ensure_password_matches_accepts_correct_password() {
        let password = test_password("p");
        let hashed = hash_password(&password).unwrap();

        assert!(ensure_password_matches(&password, &hashed).is_ok());
    }

    /// Regression: signin used `verify(..)?` and ignored the returned bool,
    /// so any password produced a valid JWT.
    #[test]
    fn test_ensure_password_matches_rejects_wrong_password() {
        let hashed = hash_password(&test_password("p")).unwrap();
        let result = ensure_password_matches(&test_password("q"), &hashed);

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
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
