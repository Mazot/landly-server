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
}
