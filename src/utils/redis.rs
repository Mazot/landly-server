use crate::{constants::env_key, error::AppError};
use actix_request_reply_cache::{RedisCacheMiddleware, RedisCacheMiddlewareBuilder};
use dotenv::dotenv;
use r2d2::Pool;
use redis::Client;
use std::env;

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
