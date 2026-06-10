use chrono::{Duration, Utc};
use mongodb::bson::DateTime;
use rand::Rng;

use crate::applications::application_store::{AppState, DevEmailEvent};
use crate::errors::ApiError;
use crate::models::{EmailLog, LoginChallenge};
use crate::queries::{auth as auth_queries, users};
use crate::utils::constants::CHALLENGE_TTL_MINUTES;
use crate::utils::security::{hash_password, verify_password};

/// Verify credentials and create a one-time, time-boxed 2FA challenge.
/// On success returns the challenge id (hex) the client uses to verify.
///
/// Returns `Unauthorized` for both unknown email and wrong password so the
/// endpoint doesn't leak which accounts exist.
pub async fn login(state: &AppState, email: &str, password: &str) -> Result<String, ApiError> {
    let invalid = || ApiError::Unauthorized("invalid email or password".into());

    let user = users::find_by_email(&state.db, email)
        .await?
        .ok_or_else(invalid)?;

    if !verify_password(password, &user.password_hash)? {
        return Err(invalid());
    }

    let user_id = user
        .id
        .ok_or_else(|| ApiError::Internal("user id missing".into()))?;

    // Generate + hash the 6-digit code (never stored in plain text).
    let code = {
        let mut rng = rand::rng();
        format!("{:06}", rng.random_range(0..1_000_000))
    };
    let code_hash = hash_password(&code)?;

    let now = Utc::now();
    let expires_at = now + Duration::minutes(CHALLENGE_TTL_MINUTES);

    let challenge = LoginChallenge {
        id: None,
        user_id,
        code_hash,
        expires_at: DateTime::from_millis(expires_at.timestamp_millis()),
        used_at: None,
        created_at: DateTime::from_millis(now.timestamp_millis()),
    };
    let challenge_id = auth_queries::insert_challenge(&state.db, &challenge).await?;

    // Audit log (masked code only).
    let masked_code = format!("****{}", &code[4..]);
    let email_log = EmailLog {
        id: None,
        to_email: user.email.clone(),
        purpose: "login_2fa".to_string(),
        masked_code,
        challenge_id,
        created_at: DateTime::from_millis(now.timestamp_millis()),
    };
    auth_queries::insert_email_log(&state.db, &email_log).await?;

    // Dev helpers: stash the real code in memory + print to console.
    {
        let mut dev_events = state.dev_email_events.write().await;
        dev_events.insert(
            user.email.clone(),
            DevEmailEvent {
                to_email: user.email.clone(),
                code: code.clone(),
                challenge_id: challenge_id.to_hex(),
                created_at_iso: now.to_rfc3339(),
            },
        );
    }
    println!(
        "[DEV EMAIL] to={} purpose=login_2fa code={} challenge_id={}",
        user.email,
        code,
        challenge_id.to_hex()
    );

    Ok(challenge_id.to_hex())
}
