use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::models::user::UserRole;

/// JWT payload issued after a successful 2FA verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

/// Authenticated caller, resolved from a verified bearer token.
/// Produced by `compute::authorization::auth_user_from_request`.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: ObjectId,
    pub email: String,
    pub role: UserRole,
}
