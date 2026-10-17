use super::redis::RedisPool;
use crate::error::AppError;
use r2d2::PooledConnection;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub trait CacheService: Send + Sync + 'static {
    fn get_string(&self, key: &str) -> Result<Option<String>, AppError>;
    fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), AppError>;
    fn delete(&self, key: &str) -> Result<(), AppError>;
    /// Atomically reads and deletes a key (Redis `GETDEL`). Use it for
    /// single-use values, where a separate get + delete would let two
    /// concurrent callers both consume the same value.
    fn take_string(&self, key: &str) -> Result<Option<String>, AppError>;
    fn exists(&self, key: &str) -> Result<bool, AppError>;
    fn invalidate_pattern(&self, pattern: &str) -> Result<(), AppError>;
    fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError>;
    fn mset_string(
        &self,
        items: &[(String, String)],
        ttl: Option<Duration>,
    ) -> Result<(), AppError>;
}

#[derive(Debug)]
pub struct TypedCache<T: ?Sized> {
    cache_service: T,
}

// ! I prefer using manual Clone implementation for TypedCache
// ! because i don't know derive will work correctly with Arc<dyn CacheService>
impl<T: ?Sized> Clone for TypedCache<Arc<T>> {
    fn clone(&self) -> Self {
        TypedCache {
            cache_service: Arc::clone(&self.cache_service),
        }
    }
}

impl<T: CacheService> TypedCache<T> {
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
            Some(json_str) => serde_json::from_str(&json_str).map(Some).map_err(|_| {
                let _ = self.delete(key);
                AppError::InternalServerError
            }),
            None => Ok(None),
        }
    }

    pub fn set<U>(&self, key: &str, value: &U, ttl: Option<Duration>) -> Result<(), AppError>
    where
        U: Serialize,
    {
        let json_str = serde_json::to_string(value).map_err(|_| AppError::InternalServerError)?;

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
                Some(json_str) => match serde_json::from_str(&json_str) {
                    Ok(parsed_value) => results.push(Some(parsed_value)),
                    Err(_) => {
                        let _ = self.delete(&keys[i]);
                        results.push(None);
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
        // A serialization failure must not panic the worker thread.
        let key_ser_val_vec: Vec<(String, String)> = items
            .iter()
            .map(|(k, v)| {
                serde_json::to_string(v)
                    .map(|s| (k.clone(), s))
                    .map_err(|_| AppError::InternalServerError)
            })
            .collect::<Result<_, _>>()?;

        self.cache_service.mset_string(&key_ser_val_vec, ttl)
    }

    pub fn delete(&self, key: &str) -> Result<(), AppError> {
        self.cache_service.delete(key)
    }

    /// Atomic get + delete of a single-use value (see `CacheService::take_string`).
    pub fn take<U>(&self, key: &str) -> Result<Option<U>, AppError>
    where
        U: for<'de> Deserialize<'de>,
    {
        match self.cache_service.take_string(key)? {
            Some(json_str) => serde_json::from_str(&json_str)
                .map(Some)
                .map_err(|_| AppError::InternalServerError),
            None => Ok(None),
        }
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
    /// TTL applied when a write passes no explicit TTL (1 hour by default).
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
            config: CacheConfig::default(),
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

    fn mset_string(
        &self,
        items: &[(String, String)],
        ttl: Option<Duration>,
    ) -> Result<(), AppError> {
        (**self).mset_string(items, ttl)
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        (**self).delete(key)
    }

    fn take_string(&self, key: &str) -> Result<Option<String>, AppError> {
        (**self).take_string(key)
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
        let res: Option<String> = connection
            .get(key)
            .map_err(|_| AppError::InternalServerError)?;

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
        let _: () = connection
            .del(key)
            .map_err(|_| AppError::InternalServerError)?;

        Ok(())
    }

    fn take_string(&self, key: &str) -> Result<Option<String>, AppError> {
        let connection = &mut self.get_connection()?;
        let res: Option<String> = redis::cmd("GETDEL")
            .arg(key)
            .query(&mut **connection)
            .map_err(|_| AppError::InternalServerError)?;

        Ok(res)
    }

    fn exists(&self, key: &str) -> Result<bool, AppError> {
        let connection = &mut self.get_connection()?;
        let exist = connection
            .exists(key)
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
            let _: () = connection
                .del(all_keys)
                .map_err(|_| AppError::InternalServerError)?;
        }

        Ok(())
    }

    fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let connection = &mut self.get_connection()?;
        let res: Vec<Option<String>> = connection
            .mget(keys)
            .map_err(|_| AppError::InternalServerError)?;

        Ok(res)
    }

    fn mset_string(
        &self,
        items: &[(String, String)],
        ttl: Option<Duration>,
    ) -> Result<(), AppError> {
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

    fn take_string(&self, _key: &str) -> Result<Option<String>, AppError> {
        Ok(None)
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

    fn mset_string(
        &self,
        _items: &[(String, String)],
        _ttl: Option<Duration>,
    ) -> Result<(), AppError> {
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

    pub fn image_by_id(id: &uuid::Uuid) -> String {
        format!("img:id:{}", id)
    }

    pub fn images_by_organisation(org_id: &uuid::Uuid) -> String {
        format!("img:org:{}", org_id)
    }

    pub fn images_pattern() -> String {
        "img:*".to_string()
    }

    pub fn common_pattern() -> String {
        "common:*".to_string()
    }

    pub fn corridors_by_user(user_id: &uuid::Uuid) -> String {
        format!("cor:user:{}", user_id)
    }

    pub fn corridor_stats(corridor_id: &uuid::Uuid) -> String {
        format!("cor:stats:{}", corridor_id)
    }

    pub fn corridor_pattern() -> String {
        "cor:*".to_string()
    }

    pub fn person_pattern() -> String {
        "per:*".to_string()
    }

    pub fn common_org_types_pattern() -> String {
        "common:organisation_types:*".to_string()
    }

    pub fn common_countries_pattern() -> String {
        "common:countries:*".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_no_op_cache_service_take() {
        assert_eq!(NoOpCacheService.take_string("k").unwrap(), None);
    }

    /// Runs against a real Redis only when REDIS_TEST_URL is set (CI has no
    /// Redis): GETDEL must hand a value out exactly once.
    #[test]
    fn test_redis_take_string_is_single_use() {
        let Ok(url) = std::env::var("REDIS_TEST_URL") else {
            return;
        };
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(redis::Client::open(url).unwrap())
            .unwrap();
        let cache = RedisCacheService::new(pool);
        let key = format!("test:take:{}", uuid::Uuid::new_v4());

        cache
            .set_string(&key, "value", Some(Duration::from_secs(60)))
            .unwrap();

        assert_eq!(cache.take_string(&key).unwrap().as_deref(), Some("value"));
        assert_eq!(cache.take_string(&key).unwrap(), None);
        assert!(!cache.exists(&key).unwrap());
    }

    #[test]
    fn test_no_op_cache_service_get() {
        let cache = NoOpCacheService;
        let result = cache.get_string("test_key");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_no_op_cache_service_set() {
        let cache = NoOpCacheService;
        let result = cache.set_string("test_key", "test_value", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_op_cache_service_delete() {
        let cache = NoOpCacheService;
        let result = cache.delete("test_key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_op_cache_service_exists() {
        let cache = NoOpCacheService;
        let result = cache.exists("test_key");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_no_op_cache_service_invalidate_pattern() {
        let cache = NoOpCacheService;
        let result = cache.invalidate_pattern("test_*");
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_op_cache_service_mget() {
        let cache = NoOpCacheService;
        let keys = vec!["key1".to_string(), "key2".to_string()];
        let result = cache.mget_string(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![None, None]);
    }

    #[test]
    fn test_no_op_cache_service_mset() {
        let cache = NoOpCacheService;
        let items = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        let result = cache.mset_string(&items, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_typed_cache_with_no_op() {
        let no_op_service = NoOpCacheService;
        let typed_cache = TypedCache::new(no_op_service);

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestData {
            value: i32,
        }

        let data = TestData { value: 42 };
        let set_result = typed_cache.set("test_key", &data, None);
        assert!(set_result.is_ok());

        let get_result: Result<Option<TestData>, AppError> = typed_cache.get("test_key");
        assert!(get_result.is_ok());
        assert_eq!(get_result.unwrap(), None);
    }

    #[test]
    fn test_typed_cache_clone() {
        let no_op_service = NoOpCacheService;
        let typed_cache = TypedCache::new(Arc::new(no_op_service) as Arc<dyn CacheService>);
        let cloned_cache = typed_cache.clone();

        let result = cloned_cache.exists("test_key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_keys_organisation_by_id() {
        let id = Uuid::new_v4();
        let key = CacheKeys::organisation_by_id(&id);
        assert_eq!(key, format!("org:id:{}", id));
    }

    #[test]
    fn test_cache_keys_organisations_list() {
        let hash = "test_hash";
        let key = CacheKeys::organisations_list(hash);
        assert_eq!(key, "org:list:test_hash");
    }

    #[test]
    fn test_cache_keys_organisation_pattern() {
        let pattern = CacheKeys::organisation_pattern();
        assert_eq!(pattern, "org:*");
    }

    #[test]
    fn test_cache_keys_organisation_count() {
        let key = CacheKeys::organisation_count();
        assert_eq!(key, "org:count");
    }

    #[test]
    fn test_cache_keys_country_connection_by_id() {
        let id = Uuid::new_v4();
        let key = CacheKeys::country_connection_by_id(&id);
        assert_eq!(key, format!("cc:id:{}", id));
    }

    #[test]
    fn test_cache_keys_country_connections_list() {
        let hash = "test_hash";
        let key = CacheKeys::country_connections_list(hash);
        assert_eq!(key, "cc:list:test_hash");
    }

    #[test]
    fn test_cache_keys_country_connection_pattern() {
        let pattern = CacheKeys::country_connection_pattern();
        assert_eq!(pattern, "cc:*");
    }

    #[test]
    fn test_cache_keys_sanitize_key() {
        assert_eq!(CacheKeys::sanitize_key("Test-Key_123"), "test-key_123");
        assert_eq!(CacheKeys::sanitize_key("Test@Key!123"), "testkey123");
        assert_eq!(CacheKeys::sanitize_key("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_cache_keys_versioned_key() {
        let key = CacheKeys::versioned_key("base_key", "v1.0");
        assert_eq!(key, "base_key:v:v1.0");
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_cache_keys_corridors_by_user() {
        let user_id = Uuid::new_v4();
        assert_eq!(
            CacheKeys::corridors_by_user(&user_id),
            format!("cor:user:{}", user_id)
        );
    }

    #[test]
    fn test_cache_keys_corridor_stats() {
        let corridor_id = Uuid::new_v4();
        assert_eq!(
            CacheKeys::corridor_stats(&corridor_id),
            format!("cor:stats:{}", corridor_id)
        );
    }

    /// The corridor pattern must cover both per-user and stats keys so a
    /// single invalidate_pattern call clears everything corridor-related.
    #[test]
    fn test_corridor_pattern_covers_corridor_keys() {
        assert_eq!(CacheKeys::corridor_pattern(), "cor:*");
        assert!(CacheKeys::corridors_by_user(&Uuid::new_v4()).starts_with("cor:"));
        assert!(CacheKeys::corridor_stats(&Uuid::new_v4()).starts_with("cor:"));
    }

    /// Every feature namespace must stay distinct — two features sharing a
    /// prefix would invalidate each other's keys on every write.
    #[test]
    fn test_feature_patterns_are_distinct() {
        let patterns = [
            CacheKeys::organisation_pattern(),
            CacheKeys::country_connection_pattern(),
            CacheKeys::images_pattern(),
            CacheKeys::corridor_pattern(),
            CacheKeys::person_pattern(),
            CacheKeys::common_pattern(),
        ];

        for (i, a) in patterns.iter().enumerate() {
            assert!(a.ends_with(":*"), "{} is not a namespace pattern", a);

            for b in patterns.iter().skip(i + 1) {
                assert_ne!(a, b);
                let (a_ns, b_ns) = (a.trim_end_matches('*'), b.trim_end_matches('*'));
                assert!(
                    !a_ns.starts_with(b_ns) && !b_ns.starts_with(a_ns),
                    "{} and {} overlap",
                    a,
                    b
                );
            }
        }
    }

    /// The common pattern must cover the request-reply cache prefixes used in
    /// common/config.rs.
    #[test]
    fn test_common_patterns() {
        assert_eq!(CacheKeys::common_pattern(), "common:*");
        assert_eq!(
            CacheKeys::common_org_types_pattern(),
            "common:organisation_types:*"
        );
        assert_eq!(CacheKeys::common_countries_pattern(), "common:countries:*");
    }

    /// Real-Redis cache behind the `Arc<dyn CacheService>` the app uses, or
    /// None without REDIS_TEST_URL (point it at a scratch DB, e.g. `/15`).
    fn redis_cache() -> Option<TypedCache<Arc<dyn CacheService>>> {
        let url = std::env::var("REDIS_TEST_URL").ok()?;
        let pool = r2d2::Pool::builder()
            .max_size(2)
            .build(redis::Client::open(url).unwrap())
            .unwrap();
        Some(TypedCache::new(
            Arc::new(RedisCacheService::new(pool)) as Arc<dyn CacheService>
        ))
    }

    #[derive(Debug, PartialEq, Serialize, serde::Deserialize, Clone)]
    struct Cached {
        id: u32,
        name: String,
    }

    #[test]
    fn test_redis_typed_get_set_delete() {
        let Some(cache) = redis_cache() else { return };
        let key = format!("test:typed:{}", uuid::Uuid::new_v4());
        let value = Cached {
            id: 7,
            name: "seven".into(),
        };

        assert_eq!(cache.get::<Cached>(&key).unwrap(), None);
        cache
            .set(&key, &value, Some(Duration::from_secs(60)))
            .unwrap();
        assert_eq!(cache.get::<Cached>(&key).unwrap(), Some(value.clone()));
        assert!(cache.exists(&key).unwrap());
        // Default TTL path (no explicit TTL).
        cache.set(&key, &value, None).unwrap();
        assert!(cache.exists(&key).unwrap());

        cache.delete(&key).unwrap();
        assert!(!cache.exists(&key).unwrap());
    }

    #[test]
    fn test_redis_mget_mset_keep_order_and_gaps() {
        let Some(cache) = redis_cache() else { return };
        let prefix = format!("test:multi:{}", uuid::Uuid::new_v4());
        let items: Vec<(String, Cached)> = (0..3)
            .map(|i| {
                (
                    format!("{prefix}:{i}"),
                    Cached {
                        id: i,
                        name: format!("n{i}"),
                    },
                )
            })
            .collect();
        cache.mset(&items, Some(Duration::from_secs(60))).unwrap();
        cache.mset::<Cached>(&[], None).unwrap();

        let keys = vec![
            items[2].0.clone(),
            format!("{prefix}:missing"),
            items[0].0.clone(),
        ];
        let got = cache.mget::<Cached>(&keys).unwrap();
        assert_eq!(
            got,
            vec![Some(items[2].1.clone()), None, Some(items[0].1.clone())]
        );
        assert!(cache.mget::<Cached>(&[]).unwrap().is_empty());

        cache.invalidate_pattern(&format!("{prefix}:*")).unwrap();
    }

    /// Repositories call this on every write; it must drop the namespace and
    /// nothing else.
    #[test]
    fn test_redis_invalidate_pattern_only_touches_the_namespace() {
        let Some(cache) = redis_cache() else { return };
        let run = uuid::Uuid::new_v4();
        let doomed: Vec<String> = (0..150).map(|i| format!("test:{run}:org:{i}")).collect();
        for key in &doomed {
            cache.set(key, &1u8, Some(Duration::from_secs(60))).unwrap();
        }
        let survivor = format!("test:{run}:cor:1");
        cache
            .set(&survivor, &1u8, Some(Duration::from_secs(60)))
            .unwrap();

        cache
            .invalidate_pattern(&format!("test:{run}:org:*"))
            .unwrap();

        assert!(
            doomed.iter().all(|k| !cache.exists(k).unwrap()),
            "SCAN must cover every page"
        );
        assert!(cache.exists(&survivor).unwrap());
        cache.invalidate_pattern(&format!("test:{run}:*")).unwrap();
        cache
            .invalidate_pattern(&format!("test:{run}:nothing:*"))
            .unwrap();
    }
}
