//! Authentication & authorization logic.
//!
//! `authenticate` turns a bearer token into an `AuthUser`; it is called by
//! `web_server::middlewares::session::SessionMiddleware` (authn at the route
//! boundary). `require_admin` is the fine-grained authz check used inside
//! admin-only handlers.

use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::models::{AuthUser, UserRole};
use crate::utils::jwt::decode_access_token;

/// Validate a JWT and build the authenticated caller.
pub fn authenticate(token: &str, jwt_secret: &str) -> Result<AuthUser, ApiError> {
    let claims = decode_access_token(token, jwt_secret)
        .map_err(|_| ApiError::Unauthorized("invalid access token".into()))?;

    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| ApiError::Unauthorized("invalid token subject".into()))?;

    let role = UserRole::from_str(&claims.role)
        .ok_or_else(|| ApiError::Unauthorized("invalid token role".into()))?;

    Ok(AuthUser {
        id: user_id,
        email: claims.email,
        role,
    })
}

/// Reject the request unless the caller is an admin.
pub fn require_admin(user: &AuthUser) -> Result<(), ApiError> {
    if user.role != UserRole::Admin {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}
