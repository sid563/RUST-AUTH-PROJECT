//! Dev-only helper: expose the latest 2FA code (kept in memory) so the login
//! flow can be exercised locally without a real email provider.

use actix_web::{get, web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::{AppState, DevEmailEvent};
use crate::errors::ApiError;
use crate::models::dtos::LatestEmailQuery;

fn event_json(event: &DevEmailEvent) -> serde_json::Value {
    json!({
        "to_email": event.to_email,
        "code": event.code,
        "challenge_id": event.challenge_id,
        "created_at": event.created_at_iso,
    })
}

#[get("/dev/email-logs/latest")]
pub async fn dev_email_logs_latest(
    state: web::Data<AppState>,
    query: web::Query<LatestEmailQuery>,
) -> Result<HttpResponse, ApiError> {
    let dev_events = state.dev_email_events.read().await;

    let event = match &query.email {
        Some(email) => dev_events.get(email),
        None => dev_events
            .values()
            .max_by(|a, b| a.created_at_iso.cmp(&b.created_at_iso)),
    };

    match event {
        Some(event) => Ok(HttpResponse::Ok().json(event_json(event))),
        None => Err(ApiError::NotFound(
            "no dev email log found for the requested email".into(),
        )),
    }
}
