use actix_web::{web, HttpResponse};

use crate::applications::application_store::AppState;
use crate::compute::tasks::view_my_tasks::view_my_tasks as view_my_tasks_compute;
use crate::errors::ApiError;
use crate::models::AuthUser;

/// `GET /tasks/view-my-tasks` (any authenticated user). Auth enforced by
/// `SessionMiddleware`; no role check.
pub async fn view_my_tasks(
    state: web::Data<AppState>,
    auth_user: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let response = view_my_tasks_compute(&state, &auth_user).await?;
    Ok(HttpResponse::Ok().json(response))
}
