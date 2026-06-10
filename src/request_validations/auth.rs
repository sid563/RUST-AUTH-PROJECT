use crate::models::dtos::{LoginRequest, Verify2faRequest};
use crate::request_validations::common::is_valid_email;

pub fn validate_login(req: &LoginRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if !is_valid_email(&req.email) {
        errors.push("email must be a valid email address".into());
    }
    if req.password.is_empty() {
        errors.push("password is required".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_verify_2fa(req: &Verify2faRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if req.login_challenge_id.trim().is_empty() {
        errors.push("login_challenge_id is required".into());
    }
    if req.code.trim().is_empty() {
        errors.push("code is required".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
