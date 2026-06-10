//! The shared application store — cloned into every request handler.

use std::{collections::HashMap, sync::Arc};

use mongodb::Database;
use tokio::sync::RwLock;

/// In-memory record of the latest 2FA code per email, used by the dev-only
/// `/dev/email-logs/latest` endpoint (local verification without real email).
#[derive(Clone)]
pub struct DevEmailEvent {
    pub to_email: String,
    pub code: String,
    pub challenge_id: String,
    pub created_at_iso: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub jwt_secret: String,
    pub redis_client: redis::Client,
    pub dev_email_events: Arc<RwLock<HashMap<String, DevEmailEvent>>>,
    /// Per-second request cap per identity; consumed by the rate-limit middleware.
    pub rate_limit_per_second: u64,
}
