//! Redis cache helpers. Thin wrappers around the connection so compute code
//! never touches the redis client directly.

use mongodb::bson::oid::ObjectId;
use redis::AsyncCommands;

use crate::errors::ApiError;
use crate::utils::constants::TASKS_VIEW_CACHE_PREFIX;

/// Per-user cache key for the `view-my-tasks` payload.
pub fn user_tasks_key(user_id: &ObjectId) -> String {
    format!("{}{}", TASKS_VIEW_CACHE_PREFIX, user_id.to_hex())
}

pub async fn get_string(client: &redis::Client, key: &str) -> Result<Option<String>, ApiError> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    let value: Option<String> = conn.get(key).await?;
    Ok(value)
}

pub async fn set_string_ex(
    client: &redis::Client,
    key: &str,
    value: &str,
    ttl_secs: u64,
) -> Result<(), ApiError> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: () = conn.set_ex(key, value, ttl_secs).await?;
    Ok(())
}

pub async fn delete(client: &redis::Client, key: &str) -> Result<(), ApiError> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: usize = conn.del(key).await?;
    Ok(())
}

/// Increment a counter and, on its first increment, set an expiry. Returns the
/// post-increment count. Used by the rate-limit middleware for per-second buckets.
pub async fn incr_with_ttl(
    client: &redis::Client,
    key: &str,
    ttl_secs: i64,
) -> Result<u64, ApiError> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    let count: u64 = conn.incr(key, 1).await?;
    if count == 1 {
        let _: () = conn.expire(key, ttl_secs).await?;
    }
    Ok(count)
}
