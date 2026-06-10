use actix_web::{post, web, HttpResponse};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::compute::auth::seed_users::{seed_users as seed, SEED_USERS};
use crate::errors::ApiError;

#[post("/seed/users")]
pub async fn seed_users(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let newly_inserted = seed(&state.db).await?;

    let seeded: Vec<_> = SEED_USERS
        .iter()
        .map(|(_, email, password, role)| {
            json!({ "email": email, "password": password, "role": role.as_str() })
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({
        "message": "seed completed",
        "seeded_users": seeded,
        "newly_inserted": newly_inserted,
    })))
}
