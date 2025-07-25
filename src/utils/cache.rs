use crate::error::AppError;
use super::redis::RedisPool;
use r2d2::PooledConnection;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub trait CacheService: Send + Sync + 'static {
    fn get_string(&self, key: &str) -> Result<Option<String>, AppError>;
    fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), AppError>;
    fn delete(&self, key: &str) -> Result<(), AppError>;
    fn exists(&self, key: &str) -> Result<bool, AppError>;
    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError>;
    fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError>;
    fn mset_string(&self, items: &[(String, String)], ttl: Option<Duration>) -> Result<(), AppError>;
}

#[derive(Debug)]
pub struct TypedCache<T: ?Sized> {
    cache_service: T,
}

// ! I prefer using manual Clone implementation for TypedCache because i don't know derive will work correctly with Arc<dyn CacheService>
impl<T: ?Sized> Clone for TypedCache<Arc<T>> {
    fn clone(&self) -> Self {
        TypedCache {
            cache_service: Arc::clone(&self.cache_service),
        }
    }
}

impl <T: CacheService> TypedCache<T> {
    pub fn new(cache_service: T) -> Self {
        Self { cache_service }
    }
}

impl<T: CacheService> TypedCache<T> {
    pub fn get<U>(&self, key: &str) -> Result<Option<U>, AppError>
    where
        U: for<'de> Deserialize<'de>,
    {
        match self.cache_service.get_string(key)? {
            Some(json_str) => {
                serde_json::from_str(&json_str)
                    .map(Some)
                    .map_err(|_| {
                        let _ = self.delete(key);
                        AppError::InternalServerError
                    })
            },
            None => Ok(None),
        }
    }

    pub fn set<U>(&self, key: &str, value: &U, ttl: Option<Duration>) -> Result<(), AppError>
    where
        U: Serialize,
    {
        let json_str = serde_json::to_string(value)
            .map_err(|_| AppError::InternalServerError)?;

        self.cache_service.set_string(key, &json_str, ttl)
    }

    pub fn mget<U>(&self, keys: &[String]) -> Result<Vec<Option<U>>, AppError>
    where
        U: for<'de> Deserialize<'de>,
    {
        let string_results = self.cache_service.mget_string(keys)?;
        let mut results = Vec::with_capacity(string_results.len());

        for (i, opt_str) in string_results.into_iter().enumerate() {
            match opt_str {
                Some(json_str) => {
                    match serde_json::from_str(&json_str) {
                        Ok(parsed_value) => results.push(Some(parsed_value)),
                        Err(_) => {
                            let _ = self.delete(&keys[i]);
                            results.push(None);
                        }
                    }
                },
                None => results.push(None),
            }
        }

        Ok(results)
    }

    pub fn mset<U>(&self, items: &[(String, U)], ttl: Option<Duration>) -> Result<(), AppError>
    where
        U: Serialize,
    {
        let key_ser_val_vec: Vec<(String, String)> = items.iter()
            .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap()))
            .collect();

        self.cache_service.mset_string(&key_ser_val_vec, ttl)
    }

    pub fn delete(&self, key: &str) -> Result<(), AppError> {
        self.cache_service.delete(key)
    }

    pub fn exists(&self, key: &str) -> Result<bool, AppError> {
        self.cache_service.exists(key)
    }

    pub fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError> {
        self.cache_service.invalidate_pattern(pattern)
    }
}


#[derive(Clone, Debug)]
pub struct RedisCacheService {
    pool: RedisPool,
    config: CacheConfig,
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub default_ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(3600)),
        }
    }
}

impl RedisCacheService {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            pool,
            config: CacheConfig::default()
        }
    }

    pub fn new_with_config(pool: RedisPool, config: CacheConfig) -> Self {
        Self { pool, config }
    }

    fn get_connection(&self) -> Result<PooledConnection<Client>, AppError> {
        self.pool.get().map_err(|_| AppError::InternalServerError)
    }
}

impl CacheService for Arc<dyn CacheService> {
    fn get_string(&self, key: &str) -> Result<Option<String>, AppError> {
        (**self).get_string(key)
    }

    fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), AppError> {
        (**self).set_string(key, value, ttl)
    }

    fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError> {
        (**self).mget_string(keys)
    }

    fn mset_string(&self, items: &[(String, String)], ttl: Option<Duration>) -> Result<(), AppError> {
        (**self).mset_string(items, ttl)
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        (**self).delete(key)
    }

    fn exists(&self, key: &str) -> Result<bool, AppError> {
        (**self).exists(key)
    }

    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError> {
        (**self).invalidate_pattern(pattern)
    }
}


impl CacheService for RedisCacheService {
    fn get_string(&self, key: &str) -> Result<Option<String>, AppError> {
        let connection = &mut self.get_connection()?;
        let res: Option<String> = connection.get(key).map_err(|_| AppError::InternalServerError)?;

        Ok(res)
    }

    fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), AppError> {
        let connection = &mut self.get_connection()?;
        let ttl_seconds = ttl.unwrap_or(self.config.default_ttl.unwrap()).as_secs();

        if ttl_seconds == 0 {
            let _: () = connection.set(key, value)?;
        } else {
            let _: () = connection.set_ex(key, value, ttl_seconds)?;
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
        let exist =  connection.exists(key)
            .map_err(|_| AppError::InternalServerError)?;

        Ok(exist)
    }

    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError> {
        /* AI GENERATED CODE */

        let connection = &mut self.get_connection()?;
        let mut cursor = 0;
        let mut all_keys = Vec::new();

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(1000)
                .query(connection)
                .map_err(|_| AppError::InternalServerError)?;

            all_keys.extend(keys);
            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        if !all_keys.is_empty() {
            let _: () = connection.del(all_keys)
                .map_err(|_| AppError::InternalServerError)?;
        }

        Ok(())
    }

    fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let connection = &mut self.get_connection()?;
        let res: Vec<Option<String>> = connection.mget(keys).map_err(|_| AppError::InternalServerError)?;

        Ok(res)
    }

    fn mset_string(&self, items: &[(String, String)], ttl: Option<Duration>) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        let connection = &mut self.get_connection()?;
        let ttl_seconds = ttl.unwrap_or(self.config.default_ttl.unwrap()).as_secs();

        if ttl_seconds == 0 {
            let _: () = connection.mset(items)?;
        } else {
            let mut pipe = redis::pipe();

            for (key, value) in items {
                pipe.set_ex(key, value, ttl_seconds);
            }

            pipe.exec(connection)
                .map_err(|_| AppError::InternalServerError)?;
        }

        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct NoOpCacheService;
impl CacheService for NoOpCacheService {
    fn get_string(&self, _key: &str) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    fn set_string(&self, _key: &str, _value: &str, _ttl: Option<Duration>) -> Result<(), AppError> {
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

    fn mget_string(&self, _keys: &[String]) -> Result<Vec<Option<String>>, AppError> {
        Ok(vec![None; _keys.len()])
    }

    fn mset_string(&self, _items: &[(String, String)], _ttl: Option<Duration>) -> Result<(), AppError> {
        Ok(())
    }
}

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

    pub fn country_connection_by_id(id: &uuid::Uuid) -> String {
        format!("cc:id:{}", id)
    }

    pub fn country_connections_list(filters_hash: &str) -> String {
        format!("cc:list:{}", filters_hash)
    }

    pub fn country_connection_pattern() -> String {
        "cc:*".to_string()
    }

    pub fn sanitize_key(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase()
    }

    pub fn versioned_key(base_key: &str, version: &str) -> String {
        format!("{}:v:{}", base_key, version)
    }
}
