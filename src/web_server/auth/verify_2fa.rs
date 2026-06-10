use actix_web::{post, web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::compute::auth::verify_2fa::verify_2fa as verify_compute;
use crate::errors::ApiError;
use crate::models::dtos::Verify2faRequest;
use crate::request_validations::auth::validate_verify_2fa;
use crate::utils::constants::ACCESS_TOKEN_TTL_SECONDS;

#[post("/auth/verify-2fa")]
pub async fn verify_2fa(
    state: web::Data<AppState>,
    body: web::Json<Verify2faRequest>,
) -> Result<HttpResponse, ApiError> {
    validate_verify_2fa(&body).map_err(ApiError::Validation)?;

    let outcome = verify_compute(&state, &body.login_challenge_id, &body.code).await?;

    Ok(HttpResponse::Ok().json(json!({
        "access_token": outcome.access_token,
        "token_type": "Bearer",
        "expires_in_seconds": ACCESS_TOKEN_TTL_SECONDS,
        "user": { "email": outcome.email, "role": outcome.role.as_str() },
    })))
}
