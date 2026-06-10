//! Request and response Data Transfer Objects — the wire shapes for the API,
//! kept separate from the MongoDB document schemas (`user`, `task`, `auth`).

use serde::{Deserialize, Serialize};

use crate::models::{TaskPriority, TaskStatus, UserRole};

// ---- Requests ---------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub login_challenge_id: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct LatestEmailQuery {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
}

#[derive(Debug, Deserialize)]
pub struct AssignTasksRequest {
    pub task_ids: Vec<String>,
    pub assignee_email: String,
}

// ---- Responses / compute outcomes -------------------------------------------

/// Result of a successful 2FA verification, returned by compute and rendered
/// by the handler into the access-token response.
#[derive(Debug, Clone)]
pub struct Verify2faOutcome {
    pub access_token: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewTaskItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewMyTasksResponse {
    pub user: serde_json::Value,
    pub tasks: Vec<ViewTaskItem>,
    pub summary: serde_json::Value,
    pub cache: serde_json::Value,
}
