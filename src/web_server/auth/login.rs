use actix_web::{post, web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::compute::auth::login::login as login_compute;
use crate::errors::ApiError;
use crate::models::dtos::LoginRequest;
use crate::request_validations::auth::validate_login;
use crate::utils::constants::CHALLENGE_TTL_MINUTES;

#[post("/auth/login")]
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    validate_login(&body).map_err(ApiError::Validation)?;

    let challenge_id = login_compute(&state, &body.email, &body.password).await?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "2fa challenge created",
        "login_challenge_id": challenge_id,
        "expires_in_seconds": CHALLENGE_TTL_MINUTES * 60,
    })))
}
