use crate::error::AppError;
use super::redis::RedisPool;
use r2d2::PooledConnection;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub trait CacheService: Send + Sync + 'static {
    fn get<T>(&self, key: &str) -> Result<Option<T>, AppError>
    where
        T: for<'de> Deserialize<'de>;

    fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize;

    fn delete(&self, key: &str) -> Result<(), AppError>;

    fn exists(&self, key: &str) -> Result<bool, AppError>;

    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError>;

    fn mget<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>, AppError>
    where
        T: for<'de> Deserialize<'de>;

    fn mset<T>(&self, items: &[(String, T)], ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize;
}

#[derive(Clone, Debug)]
pub struct RedisCacheService {
    pool: RedisPool,
    config: CacheConfig,
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    // TODO: Needt to fill config fields
    pub default_ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(60)), // Default TTL of 60 seconds
        }
    }
}

impl RedisCacheService {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool, config: CacheConfig::default() }
    }

    pub fn new_with_config(pool: RedisPool, config: CacheConfig) -> Self {
        Self { pool, config }
    }

    pub fn get_connection(&self) -> Result<PooledConnection<Client>, AppError> {
        self.pool.get().map_err(|_| AppError::InternalServerError)
    }
}

impl CacheService for RedisCacheService {
    fn get<T>(&self, key: &str) -> Result<Option<T>, AppError>
    where
        T: for<'de> Deserialize<'de>
    {
        let connection = &mut self.get_connection()?;
        let res: String = connection.get(key).map_err(|_| AppError::InternalServerError)?;

        if res.is_empty() {
            Ok(None)
        } else {
            match serde_json::from_str(&res) {
                Ok(value) => Ok(Some(value)),
                Err(_) => {
                    let _ = self.delete(key)?;
                    Err(AppError::InternalServerError)
                }
            }
        }
    }

    fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize
    {
        let connection = &mut self.get_connection()?;
        let ttl_seconds = ttl.map_or(
            self.config.default_ttl.unwrap().as_secs(), |d| d.as_secs()
        );
        let serialized_value = serde_json::to_string(value)
            .map_err(|_| AppError::InternalServerError)?;

        if ttl_seconds == 0 {
            let _: () = connection.set(key, serialized_value)?;
        } else {
            let _: () = connection.set_ex(key, serialized_value, ttl_seconds)?;
        }

        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        let connection = &mut self.get_connection()?;
        let _: () = connection.del(key).map_err(|_| AppError::InternalServerError)?;

        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool, AppError> {
        let connection = &mut self.get_connection()?;
        let exists: bool = connection.exists(key).map_err(|_| AppError::InternalServerError)?;

        Ok(exists)
    }

    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError> {
        /* AI GENERATED CODE */

        let pattern_clone = pattern.to_string();
        let mut connection = self.get_connection()?;

        // Get all keys matching pattern using SCAN
        let mut cursor = 0;
        let mut all_keys = Vec::new();

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern_clone)
                .arg("COUNT")
                .arg(1000)
                .query(&mut connection)?;

            all_keys.extend(keys);
            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        if !all_keys.is_empty() {
            let _: () = connection.del(all_keys)?;
        }

        Ok(())
    }

    fn mget<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>, AppError>
    where
        T: for<'de> Deserialize<'de>
    {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let keys_vec = keys.to_vec();
        let connection = &mut self.get_connection()?;
        let raw_res_vec: Vec<Option<String>> = connection.mget(&keys_vec)?;
        let mut results: Vec<Option<T>> = Vec::with_capacity(keys.len());

        for (i, res) in raw_res_vec.iter().enumerate() {
            match res {
                Some(value) => {
                    match serde_json::from_str(value) {
                        Ok(parsed_value) => results.push(parsed_value),
                        Err(_) => {
                            self.delete(&keys_vec[i])?;
                            results.push(None);
                        },
                    }
                },
                None => results.push(None),
            }
        }

        Ok(results)
    }

    fn mset<T>(&self, items: &[(String, T)], ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize
    {
        if items.is_empty() {
            return Ok(());
        }

        let connection = &mut self.get_connection()?;
        let ttl_seconds = ttl.map_or(
            self.config.default_ttl.unwrap().as_secs(), |d| d.as_secs()
        );

        if ttl_seconds == 0 {
            let key_ser_val_vec: Vec<(String, String)> = items.iter()
                .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap()))
                .collect();
            let _: () = connection.mset(&key_ser_val_vec)?;
        } else {
            let mut pipe = redis::pipe();

            for (key, value) in items {
                let serialized_value = serde_json::to_string(value)
                    .map_err(|_| AppError::InternalServerError)?;
                pipe.set_ex(key, serialized_value, ttl_seconds);
            }

            pipe.exec(connection)
                .map_err(|_| AppError::InternalServerError)?;
        }

        Ok(())
    }
}


// No-op cache implementation for fallback
#[derive(Clone)]
pub struct NoOpCacheService;

impl CacheService for NoOpCacheService {
    fn get<T>(&self, _key: &str) -> Result<Option<T>, AppError>
    where
        T: for<'de> Deserialize<'de>
    {
        Ok(None)
    }

    fn set<T>(&self, _key: &str, _value: &T, _ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize
    {
        Ok(())
    }

    fn delete(&self, _key: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn exists(&self, _key: &str) -> Result<bool, AppError> {
        Ok(false)
    }

    fn invalidate_pattern(&self, _pattern: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn mget<T>(&self, _keys: &[String]) -> Result<Vec<Option<T>>, AppError>
    where
        T: for<'de> Deserialize<'de>
    {
        let mut result = Vec::with_capacity(_keys.len());
        for _ in 0.._keys.len() {
            result.push(None);
        }

        Ok(result)
    }

    fn mset<T>(&self, _items: &[(String, T)], _ttl: Option<Duration>) -> Result<(), AppError>
    where
        T: Serialize
    {
        Ok(())
    }
}

// Cache key builders for consistent naming
pub struct CacheKeys;

impl CacheKeys {
    pub fn organisation_by_id(id: &uuid::Uuid) -> String {
        format!("org:id:{}", id)
    }

    pub fn organisations_list(filters_hash: &str) -> String {
        format!("org:list:{}", filters_hash)
    }

    pub fn organisation_pattern() -> String {
        "org:*".to_string()
    }

    pub fn organisation_count() -> String {
        "org:count".to_string()
    }

    // Country connection cache keys
    pub fn country_connection_by_id(id: &uuid::Uuid) -> String {
        format!("cc:id:{}", id)
    }

    pub fn country_connections_list(filters_hash: &str) -> String {
        format!("cc:list:{}", filters_hash)
    }

    pub fn country_connection_pattern() -> String {
        "cc:*".to_string()
    }

    // Utility methods
    fn sanitize_key(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase()
    }

    // Generate versioned keys for cache invalidation
    pub fn versioned_key(base_key: &str, version: &str) -> String {
        format!("{}:v:{}", base_key, version)
    }
}
