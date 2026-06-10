use mongodb::bson::{oid::ObjectId, DateTime};

use crate::applications::application_store::AppState;
use crate::errors::ApiError;
use crate::models::dtos::Verify2faOutcome;
use crate::queries::{auth as auth_queries, users};
use crate::utils::jwt::issue_access_token;
use crate::utils::security::verify_password;

/// Verify a 2FA code against its challenge and, on success, issue an access
/// token. Enforces: challenge exists, not already used, not expired, code matches.
pub async fn verify_2fa(
    state: &AppState,
    login_challenge_id: &str,
    code: &str,
) -> Result<Verify2faOutcome, ApiError> {
    let challenge_id = ObjectId::parse_str(login_challenge_id)
        .map_err(|_| ApiError::BadRequest("invalid login_challenge_id".into()))?;

    let challenge = auth_queries::find_challenge_by_id(&state.db, &challenge_id)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("invalid or expired challenge".into()))?;

    if challenge.used_at.is_some() {
        return Err(ApiError::Unauthorized("verification code already used".into()));
    }
    if challenge.expires_at < DateTime::now() {
        return Err(ApiError::Unauthorized("verification code expired".into()));
    }
    if !verify_password(code, &challenge.code_hash)? {
        return Err(ApiError::Unauthorized("incorrect verification code".into()));
    }

    // Consume the challenge (one-time use) before issuing the token.
    auth_queries::mark_challenge_used(&state.db, &challenge_id, DateTime::now()).await?;

    let user = users::find_by_id(&state.db, &challenge.user_id)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("user not found for challenge".into()))?;
    let user_id = user
        .id
        .ok_or_else(|| ApiError::Internal("user id missing".into()))?;

    let access_token = issue_access_token(&user_id, &user.email, &user.role, &state.jwt_secret)?;

    Ok(Verify2faOutcome {
        access_token,
        email: user.email,
        role: user.role,
    })
}
