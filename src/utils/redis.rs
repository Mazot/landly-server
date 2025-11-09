use crate::{constants::env_key, error::AppError};
use actix_request_reply_cache::{RedisCacheMiddleware, RedisCacheMiddlewareBuilder};
use dotenv::dotenv;
use std::env;
use r2d2::Pool;
use redis::Client;

pub type RedisPool = Pool<Client>;

pub fn establish_connection() -> Result<RedisPool, AppError> {
    dotenv().ok();
    let redis_url = env::var(env_key::REDIS_URL)?;
    let client = Client::open(redis_url)?;

    // TODO: Need to add connection pool configuration
    let pool = Pool::builder()
        .test_on_check_out(true)
        .max_size(10)
        .build(client)?;

    Ok(pool)
}

pub fn make_common_get_request_cache(cache_prefix: &str, ttl: u64) -> RedisCacheMiddleware {
    dotenv().ok();
    let redis_url = env::var(env_key::REDIS_URL)
        .expect("REDIS_URL must be set");
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
}
