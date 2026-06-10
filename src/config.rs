use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub mongo_uri: String,
    pub mongo_db_name: String,
    pub redis_url: String,
    pub jwt_secret: String,
    /// Max requests allowed per identity (user, else IP) within a 1-second
    /// bucket. `0` disables rate limiting.
    pub rate_limit_per_second: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let mongo_uri = env::var("MONGO_URI")
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let mongo_db_name = env::var("MONGO_DB_NAME").unwrap_or_else(|_| "task_auth_db".to_string());
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-env".to_string());
        let rate_limit_per_second = env::var("RATE_LIMIT_PER_SECOND")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);

        Self {
            host,
            port,
            mongo_uri,
            mongo_db_name,
            redis_url,
            jwt_secret,
            rate_limit_per_second,
        }
    }
}
