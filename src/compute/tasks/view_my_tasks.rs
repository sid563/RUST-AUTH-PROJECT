use serde_json::json;

use crate::applications::application_store::AppState;
use crate::errors::ApiError;
use crate::models::dtos::{ViewMyTasksResponse, ViewTaskItem};
use crate::models::AuthUser;
use crate::queries::tasks;
use crate::utils::cache;
use crate::utils::constants::TASKS_VIEW_CACHE_TTL_SECS;

/// Return the caller's assigned tasks, served from the per-user Redis cache
/// when warm. On a miss, query Mongo, build the payload, cache it, and return
/// it with `cache.hit = false`.
pub async fn view_my_tasks(
    state: &AppState,
    auth_user: &AuthUser,
) -> Result<ViewMyTasksResponse, ApiError> {
    let cache_key = cache::user_tasks_key(&auth_user.id);

    // Cache hit: decode and flip the cache flag.
    if let Some(cached) = cache::get_string(&state.redis_client, &cache_key).await? {
        let mut response: ViewMyTasksResponse = serde_json::from_str(&cached)?;
        response.cache = json!({ "hit": true });
        return Ok(response);
    }

    // Cache miss: load from Mongo.
    let mut tasks = tasks::find_assigned_to(&state.db, &auth_user.id).await?;
    tasks.sort_by_key(|t| t.created_at);

    let response_tasks = tasks
        .iter()
        .map(|task| ViewTaskItem {
            id: task.id.map(|v| v.to_hex()).unwrap_or_default(),
            title: task.title.clone(),
            status: task.status.as_str().to_string(),
            priority: task.priority.as_str().to_string(),
            assigned_to: auth_user.email.clone(),
        })
        .collect::<Vec<_>>();

    let response = ViewMyTasksResponse {
        user: json!({ "email": auth_user.email, "role": auth_user.role.as_str() }),
        tasks: response_tasks,
        summary: json!({ "total_assigned_tasks": tasks.len() }),
        cache: json!({ "hit": false }),
    };

    let encoded = serde_json::to_string(&response)?;
    cache::set_string_ex(
        &state.redis_client,
        &cache_key,
        &encoded,
        TASKS_VIEW_CACHE_TTL_SECS,
    )
    .await?;

    Ok(response)
}
