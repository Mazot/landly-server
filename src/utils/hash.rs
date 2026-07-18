pub use bcrypt::verify;
use bcrypt::{BcryptResult, DEFAULT_COST, hash};

/// Hashes a plaintext password with bcrypt at the default cost.
/// Verification goes through the re-exported `verify`.
pub fn hash_password(naive_password: &str) -> BcryptResult<String> {
    hash(naive_password, DEFAULT_COST)
}

#[cfg(test)]
mod tests {
    use super::*;

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
