use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::compute::authorization::require_admin;
use crate::compute::tasks::create_task::create_task as create_task_compute;
use crate::errors::ApiError;
use crate::models::dtos::CreateTaskRequest;
use crate::models::AuthUser;
use crate::request_validations::tasks::validate_create_task;

/// `POST /tasks` (admin only). Authentication is enforced by `SessionMiddleware`
/// at the route boundary; `auth_user` is injected from the request extensions.
pub async fn create_task(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    body: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, ApiError> {
    require_admin(&auth_user)?;
    validate_create_task(&body).map_err(ApiError::Validation)?;

    let task_id = create_task_compute(&state, &auth_user, &body).await?;

    Ok(HttpResponse::Ok().json(json!({ "message": "task created", "task_id": task_id })))
}
