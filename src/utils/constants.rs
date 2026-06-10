//! Global constant values: collection names, cache keys/TTLs, token lifetimes.

// MongoDB collection names
pub const USERS_COLLECTION: &str = "users";
pub const TASKS_COLLECTION: &str = "tasks";
pub const LOGIN_CHALLENGES_COLLECTION: &str = "login_challenges";
pub const EMAIL_LOGS_COLLECTION: &str = "email_logs";

// Redis cache
pub const TASKS_VIEW_CACHE_PREFIX: &str = "tasks:view:";
pub const TASKS_VIEW_CACHE_TTL_SECS: u64 = 300;

// Auth lifetimes
pub const CHALLENGE_TTL_MINUTES: i64 = 5;
pub const ACCESS_TOKEN_TTL_HOURS: i64 = 24;
pub const ACCESS_TOKEN_TTL_SECONDS: i64 = ACCESS_TOKEN_TTL_HOURS * 3600;
