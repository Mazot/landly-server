pub const BIND: &str = "0.0.0.0:8080";

pub mod env_key {
    pub const FRONTEND_ORIGIN: &str = "FRONTEND_ORIGIN";
    pub const DATABASE_URL: &str = "DATABASE_URL";
    pub const REDIS_URL: &str = "REDIS_URL";
    pub const GOOGLE_CLIENT_ID: &str = "GOOGLE_CLIENT_ID";
    pub const GOOGLE_CLIENT_SECRET: &str = "GOOGLE_CLIENT_SECRET";
    pub const OAUTH_GOOGLE_REDIRECT_URL: &str = "OAUTH_GOOGLE_REDIRECT_URL";
    pub const JWT_SECRET: &str = "JWT_SECRET";
    pub const JWT_EXPIRATION: &str = "JWT_EXPIRATION";

    // Redis Pool Configuration
    pub const REDIS_POOL_MAX_SIZE: &str = "REDIS_POOL_MAX_SIZE";
    pub const REDIS_POOL_MIN_IDLE: &str = "REDIS_POOL_MIN_IDLE";
    pub const REDIS_POOL_MAX_LIFETIME_SECS: &str = "REDIS_POOL_MAX_LIFETIME_SECS";
    pub const REDIS_POOL_IDLE_TIMEOUT_SECS: &str = "REDIS_POOL_IDLE_TIMEOUT_SECS";
    pub const REDIS_POOL_CONNECTION_TIMEOUT_SECS: &str = "REDIS_POOL_CONNECTION_TIMEOUT_SECS";

    // Database Pool Configuration
    pub const DB_POOL_MAX_SIZE: &str = "DB_POOL_MAX_SIZE";
    pub const DB_POOL_MIN_IDLE: &str = "DB_POOL_MIN_IDLE";
    pub const DB_POOL_MAX_LIFETIME_SECS: &str = "DB_POOL_MAX_LIFETIME_SECS";
    pub const DB_POOL_IDLE_TIMEOUT_SECS: &str = "DB_POOL_IDLE_TIMEOUT_SECS";
    pub const DB_POOL_CONNECTION_TIMEOUT_SECS: &str = "DB_POOL_CONNECTION_TIMEOUT_SECS";
}
