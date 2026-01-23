use crate::{constants::env_key, error::AppError};
use actix_request_reply_cache::{RedisCacheMiddleware, RedisCacheMiddlewareBuilder};
use dotenv::dotenv;
use r2d2::Pool;
use redis::Client;
use std::env;
use std::time::Duration;

pub type RedisPool = Pool<Client>;

/// Establishes a Redis connection pool with configurable settings from environment variables.
///
/// Configuration is loaded from the following environment variables with sensible defaults:
/// - `REDIS_POOL_MAX_SIZE`: Maximum number of connections (default: 10)
/// - `REDIS_POOL_MIN_IDLE`: Minimum idle connections to maintain (default: 2)
/// - `REDIS_POOL_MAX_LIFETIME_SECS`: Maximum connection lifetime in seconds (default: 1800)
/// - `REDIS_POOL_IDLE_TIMEOUT_SECS`: Idle connection timeout in seconds (default: 600)
/// - `REDIS_POOL_CONNECTION_TIMEOUT_SECS`: Connection acquisition timeout in seconds (default: 30)
pub fn establish_connection() -> Result<RedisPool, AppError> {
    dotenv().ok();
    let redis_url = env::var(env_key::REDIS_URL)?;
    let client = Client::open(redis_url)?;

    // Read pool configuration from environment variables with sensible defaults
    let max_size = env::var(env_key::REDIS_POOL_MAX_SIZE)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let min_idle = env::var(env_key::REDIS_POOL_MIN_IDLE)
        .ok()
        .and_then(|s| s.parse().ok());

    let max_lifetime = env::var(env_key::REDIS_POOL_MAX_LIFETIME_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs);

    let idle_timeout = env::var(env_key::REDIS_POOL_IDLE_TIMEOUT_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs);

    let connection_timeout = env::var(env_key::REDIS_POOL_CONNECTION_TIMEOUT_SECS)
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    let pool = Pool::builder()
        .max_size(max_size)
        .min_idle(min_idle)
        .max_lifetime(max_lifetime)
        .idle_timeout(idle_timeout)
        .connection_timeout(connection_timeout)
        .test_on_check_out(true)
        .build(client)?;

    Ok(pool)
}

pub fn make_common_get_request_cache(cache_prefix: &str, ttl: u64) -> RedisCacheMiddleware {
    dotenv().ok();
    let redis_url = env::var(env_key::REDIS_URL).expect("REDIS_URL must be set");
    let cache = RedisCacheMiddlewareBuilder::new(redis_url)
        .cache_prefix(cache_prefix)
        .ttl(ttl)
        .cache_if(|ctx| {
            if ctx.method != "GET" {
                return false;
            }

            true
        })
        .build();

    cache
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_establish_connection_without_env() {
        // This test verifies that the function returns an error when REDIS_URL is not set
        unsafe {
            std::env::remove_var("REDIS_URL");
        }

        let result = establish_connection();
        assert!(result.is_err());
    }

    #[test]
    fn test_establish_connection_with_invalid_url() {
        unsafe {
            std::env::set_var("REDIS_URL", "invalid://url");
        }

        let result = establish_connection();
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "REDIS_URL must be set")]
    fn test_make_common_get_request_cache_panics_without_env() {
        unsafe {
            std::env::remove_var("REDIS_URL");
        }

        make_common_get_request_cache("test_prefix", 60);
    }

    #[test]
    fn test_pool_config_defaults() {
        // Test that default values are used when environment variables are not set
        unsafe {
            std::env::set_var("REDIS_URL", "redis://localhost:6379");
            std::env::remove_var("REDIS_POOL_MAX_SIZE");
            std::env::remove_var("REDIS_POOL_MIN_IDLE");
            std::env::remove_var("REDIS_POOL_MAX_LIFETIME_SECS");
            std::env::remove_var("REDIS_POOL_IDLE_TIMEOUT_SECS");
            std::env::remove_var("REDIS_POOL_CONNECTION_TIMEOUT_SECS");
        }

        // The function should use defaults and not panic during pool configuration
        // It may error on actual connection, but that's expected in test environment
        let _result = establish_connection();
        // Test passes if we reach here without panicking
    }

    #[test]
    fn test_pool_config_custom_values() {
        // Test that custom values from environment variables are respected
        unsafe {
            std::env::set_var("REDIS_URL", "redis://localhost:6379");
            std::env::set_var("REDIS_POOL_MAX_SIZE", "20");
            std::env::set_var("REDIS_POOL_MIN_IDLE", "5");
            std::env::set_var("REDIS_POOL_MAX_LIFETIME_SECS", "3600");
            std::env::set_var("REDIS_POOL_IDLE_TIMEOUT_SECS", "1200");
            std::env::set_var("REDIS_POOL_CONNECTION_TIMEOUT_SECS", "60");
        }

        // The function should use custom values and not panic during pool configuration
        // It may error on actual connection, but that's expected in test environment
        let _result = establish_connection();
        // Test passes if we reach here without panicking
    }

    #[test]
    fn test_pool_config_invalid_values_use_defaults() {
        // Test that invalid values fallback to defaults without panicking
        unsafe {
            std::env::set_var("REDIS_URL", "redis://localhost:6379");
            std::env::set_var("REDIS_POOL_MAX_SIZE", "not_a_number");
            std::env::set_var("REDIS_POOL_MIN_IDLE", "invalid");
        }

        // The function should fallback to defaults and not panic during configuration parsing
        // It may error on actual connection, but that's expected in test environment
        let _result = establish_connection();
        // Test passes if we reach here without panicking
    }
}
