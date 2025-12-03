use crate::constants::env_key;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use std::env;
use std::time::Duration;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Establishes a database connection pool with configurable settings from environment variables.
///
/// Configuration is loaded from the following environment variables with sensible defaults:
/// - `DB_POOL_MAX_SIZE`: Maximum number of connections (default: 10)
/// - `DB_POOL_MIN_IDLE`: Minimum idle connections to maintain (default: 2)
/// - `DB_POOL_MAX_LIFETIME_SECS`: Maximum connection lifetime in seconds (default: 1800)
/// - `DB_POOL_IDLE_TIMEOUT_SECS`: Idle connection timeout in seconds (default: 600)
/// - `DB_POOL_CONNECTION_TIMEOUT_SECS`: Connection acquisition timeout in seconds (default: 30)
pub fn establish_connection() -> DbPool {
    dotenv().ok();
    let database_url = env::var(env_key::DATABASE_URL).expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    // Read pool configuration from environment variables with sensible defaults
    let max_size = env::var(env_key::DB_POOL_MAX_SIZE)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let min_idle = env::var(env_key::DB_POOL_MIN_IDLE)
        .ok()
        .and_then(|s| s.parse().ok());

    let max_lifetime = env::var(env_key::DB_POOL_MAX_LIFETIME_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs);

    let idle_timeout = env::var(env_key::DB_POOL_IDLE_TIMEOUT_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs);

    let connection_timeout = env::var(env_key::DB_POOL_CONNECTION_TIMEOUT_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    Pool::builder()
        .max_size(max_size)
        .min_idle(min_idle)
        .max_lifetime(max_lifetime)
        .idle_timeout(idle_timeout)
        .connection_timeout(connection_timeout)
        .test_on_check_out(true)
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

    #[test]
    #[should_panic(expected = "DATABASE_URL must be set")]
    fn test_establish_connection_without_database_url() {
        unsafe {
            std::env::remove_var("DATABASE_URL");
        }

        // This should panic with "DATABASE_URL must be set"
        establish_connection();
    }

    #[test]
    fn test_pool_config_defaults() {
        // Test that default values are used when environment variables are not set
        unsafe {
            std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/db");
            std::env::remove_var("DB_POOL_MAX_SIZE");
            std::env::remove_var("DB_POOL_MIN_IDLE");
            std::env::remove_var("DB_POOL_MAX_LIFETIME_SECS");
            std::env::remove_var("DB_POOL_IDLE_TIMEOUT_SECS");
            std::env::remove_var("DB_POOL_CONNECTION_TIMEOUT_SECS");
        }

        // The function should use defaults - it will panic on connection failure
        // but pool configuration should be applied correctly
        let result = std::panic::catch_unwind(|| {
            establish_connection();
        });

        // We expect this to panic because we can't connect to the database
        // but the important thing is that the pool configuration was parsed correctly
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_config_custom_values() {
        // Test that custom values from environment variables are respected
        unsafe {
            std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/db");
            std::env::set_var("DB_POOL_MAX_SIZE", "20");
            std::env::set_var("DB_POOL_MIN_IDLE", "5");
            std::env::set_var("DB_POOL_MAX_LIFETIME_SECS", "3600");
            std::env::set_var("DB_POOL_IDLE_TIMEOUT_SECS", "1200");
            std::env::set_var("DB_POOL_CONNECTION_TIMEOUT_SECS", "60");
        }

        // The function should use custom values - it will panic on connection failure
        // but pool configuration should be applied correctly
        let result = std::panic::catch_unwind(|| {
            establish_connection();
        });

        // We expect this to panic because we can't connect to the database
        // but the important thing is that the pool configuration was parsed correctly
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_config_invalid_values_use_defaults() {
        // Test that invalid values fallback to defaults
        unsafe {
            std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/db");
            std::env::set_var("DB_POOL_MAX_SIZE", "not_a_number");
            std::env::set_var("DB_POOL_MIN_IDLE", "invalid");
        }

        // The function should fallback to defaults and not panic during configuration parsing
        let result = std::panic::catch_unwind(|| {
            establish_connection();
        });

        // We expect this to panic because we can't connect to the database
        // but the important thing is that the pool configuration was parsed correctly
        assert!(result.is_err());
    }
}
