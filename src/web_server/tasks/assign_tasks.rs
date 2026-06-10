use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::compute::authorization::require_admin;
use crate::compute::tasks::assign_tasks::assign_tasks as assign_tasks_compute;
use crate::errors::ApiError;
use crate::models::dtos::AssignTasksRequest;
use crate::models::AuthUser;
use crate::request_validations::tasks::validate_assign_tasks;

/// `POST /tasks/assign` (admin only). Auth enforced by `SessionMiddleware`.
pub async fn assign_tasks(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    body: web::Json<AssignTasksRequest>,
) -> Result<HttpResponse, ApiError> {
    require_admin(&auth_user)?;
    validate_assign_tasks(&body).map_err(ApiError::Validation)?;

    let task_count = assign_tasks_compute(&state, &body).await?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "tasks assigned",
        "assigned_to": body.assignee_email,
        "task_count": task_count,
    })))
}
