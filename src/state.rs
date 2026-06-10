use std::{collections::HashMap, sync::Arc};

use mongodb::Database;
use tokio::sync::RwLock;

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
}
