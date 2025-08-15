pub use bcrypt::verify;
use bcrypt::{hash, BcryptResult, DEFAULT_COST};

pub fn hash_password(naive_password: &str) -> BcryptResult<String> {
    hash(naive_password, DEFAULT_COST)
}
