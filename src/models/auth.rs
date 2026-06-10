use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

/// One-time 2FA challenge created at login, verified at `/auth/verify-2fa`.
/// The code is stored hashed (`code_hash`), never in plain text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginChallenge {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub code_hash: String,
    pub expires_at: DateTime,
    pub used_at: Option<DateTime>,
    pub created_at: DateTime,
}

/// Audit record of a 2FA email "send". The real code is never persisted here —
/// only a masked form. (The dev-only console/endpoint exposes the real code.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLog {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub to_email: String,
    pub purpose: String,
    pub masked_code: String,
    pub challenge_id: ObjectId,
    pub created_at: DateTime,
}
