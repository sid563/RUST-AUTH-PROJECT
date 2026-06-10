use crate::models::dtos::{AssignTasksRequest, CreateTaskRequest};
use crate::request_validations::common::is_valid_email;

pub fn validate_create_task(req: &CreateTaskRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if req.title.trim().is_empty() {
        errors.push("title is required".into());
    }
    if req.description.trim().is_empty() {
        errors.push("description is required".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_assign_tasks(req: &AssignTasksRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if req.task_ids.is_empty() {
        errors.push("task_ids cannot be empty".into());
    }
    if !is_valid_email(&req.assignee_email) {
        errors.push("assignee_email must be a valid email address".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
