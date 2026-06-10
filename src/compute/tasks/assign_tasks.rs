use mongodb::bson::{oid::ObjectId, DateTime};

use crate::applications::application_store::AppState;
use crate::errors::ApiError;
use crate::models::dtos::AssignTasksRequest;
use crate::queries::{tasks, users};
use crate::utils::cache;

/// Assign tasks to a user by email, then invalidate the task-view cache for
/// every affected user (the new assignee plus any previous assignees).
/// Returns the number of tasks assigned.
pub async fn assign_tasks(state: &AppState, req: &AssignTasksRequest) -> Result<usize, ApiError> {
    if req.task_ids.is_empty() {
        return Err(ApiError::BadRequest("task_ids cannot be empty".into()));
    }

    let assignee = users::find_by_email(&state.db, &req.assignee_email)
        .await?
        .ok_or_else(|| ApiError::NotFound("assignee user not found".into()))?;
    let assignee_id = assignee
        .id
        .ok_or_else(|| ApiError::Internal("assignee id missing".into()))?;

    // Parse + validate all ids up front.
    let mut task_object_ids = Vec::with_capacity(req.task_ids.len());
    for raw_id in &req.task_ids {
        let parsed = ObjectId::parse_str(raw_id)
            .map_err(|_| ApiError::BadRequest(format!("invalid task id: {raw_id}")))?;
        task_object_ids.push(parsed);
    }

    let existing_tasks = tasks::find_by_ids(&state.db, &task_object_ids).await?;
    if existing_tasks.len() != task_object_ids.len() {
        return Err(ApiError::BadRequest("one or more task_ids do not exist".into()));
    }

    tasks::assign_to(&state.db, &task_object_ids, &assignee_id, DateTime::now()).await?;

    // Invalidate cache for the assignee and any prior assignees.
    let mut users_to_invalidate = vec![assignee_id];
    for task in &existing_tasks {
        if let Some(prev_user_id) = task.assigned_to_id {
            if !users_to_invalidate.contains(&prev_user_id) {
                users_to_invalidate.push(prev_user_id);
            }
        }
    }
    for user_id in &users_to_invalidate {
        cache::delete(&state.redis_client, &cache::user_tasks_key(user_id)).await?;
    }

    Ok(task_object_ids.len())
}
