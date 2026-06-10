use mongodb::bson::DateTime;

use crate::applications::application_store::AppState;
use crate::errors::ApiError;
use crate::models::dtos::CreateTaskRequest;
use crate::models::{AuthUser, Task};
use crate::queries::tasks;

/// Create a task authored by `auth_user`. Caller must already be authorized
/// as admin (see `authorization::require_admin`). Returns the new task id (hex).
pub async fn create_task(
    state: &AppState,
    auth_user: &AuthUser,
    req: &CreateTaskRequest,
) -> Result<String, ApiError> {
    let now = DateTime::now();
    let task = Task {
        id: None,
        title: req.title.clone(),
        description: req.description.clone(),
        status: req.status.clone(),
        priority: req.priority.clone(),
        created_by_id: auth_user.id,
        assigned_to_id: None,
        created_at: now,
        updated_at: now,
    };

    let task_id = tasks::insert(&state.db, &task).await?;
    Ok(task_id.to_hex())
}
