//! Input validation layer. Validators run in `web_server/` handlers *before*
//! calling `compute/`. Each returns `Result<(), Vec<String>>` so all field
//! errors are collected and reported at once (rendered as `ApiError::Validation`).

pub mod auth;
pub mod common;
pub mod tasks;
