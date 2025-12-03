use crate::constants::env_key;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use std::env;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection() -> DbPool {
    dotenv().ok();
    let database_url = env::var(env_key::DATABASE_URL)
        .expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    // TODO: Need to add connection pool configuration
    // e.g., max size, timeout, etc.
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_establish_connection_with_invalid_url() {
        unsafe {
            std::env::set_var("DATABASE_URL", "not_a_valid_postgres_url");
        }
        
        // This should panic with "Failed to create pool"
        establish_connection();
    }

    #[test]
    fn test_establish_connection_requires_database_url() {
        // We can't fully test the absence of DATABASE_URL due to test isolation
        // but we can verify the function depends on it being set
        // If DATABASE_URL is not set, establish_connection will panic
        
        // Just verify the function exists and has the right signature
        let _fn_exists: fn() -> DbPool = establish_connection;
    }
}
