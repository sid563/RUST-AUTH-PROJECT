//! Custom error types.
//!
//! One `ApiError` enum is shared across layers. `queries/` and `compute/`
//! return `Result<T, ApiError>`; because `ApiError` implements actix-web's
//! `ResponseError`, `web_server/` handlers can return `Result<_, ApiError>`
//! directly and actix renders the correct status + JSON body.
//!
//! As the project grows, add per-domain error enums (e.g. `order_errors.rs`,
//! `shipment_errors.rs`) and convert them into `ApiError` via `From`.

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("forbidden")]
    Forbidden,

    #[error("{0}")]
    NotFound(String),

    /// Aggregated input-validation failures (see `request_validations/`).
    #[error("validation failed")]
    Validation(Vec<String>),

    #[error("{0}")]
    Internal(String),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) | ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let body = match self {
            ApiError::Validation(errors) => json!({ "error": "validation failed", "details": errors }),
            other => json!({ "error": other.to_string() }),
        };
        HttpResponse::build(self.status_code()).json(body)
    }
}

// ---- Conversions from lower-level errors → Internal -------------------------

impl From<mongodb::error::Error> for ApiError {
    fn from(e: mongodb::error::Error) -> Self {
        ApiError::Internal(format!("db error: {e}"))
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(e: redis::RedisError) -> Self {
        ApiError::Internal(format!("redis error: {e}"))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Internal(format!("serialization error: {e}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}
