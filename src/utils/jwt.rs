//! JWT issue/decode helpers built on `jsonwebtoken`.

use anyhow::anyhow;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use mongodb::bson::oid::ObjectId;

use crate::models::{JwtClaims, UserRole};
use crate::utils::constants::ACCESS_TOKEN_TTL_HOURS;

pub fn issue_access_token(
    user_id: &ObjectId,
    email: &str,
    role: &UserRole,
    jwt_secret: &str,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let exp = now + Duration::hours(ACCESS_TOKEN_TTL_HOURS);

    let claims = JwtClaims {
        sub: user_id.to_hex(),
        email: email.to_string(),
        role: role.as_str().to_string(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| anyhow!("failed to encode jwt: {e}"))
}

pub fn decode_access_token(token: &str, jwt_secret: &str) -> anyhow::Result<JwtClaims> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow!("failed to decode jwt: {e}"))?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::oid::ObjectId;

    use crate::models::user::UserRole;

    use super::{decode_access_token, issue_access_token};

    #[test]
    fn issues_and_decodes_token() {
        let user_id = ObjectId::new();
        let secret = "test-secret";
        let token = issue_access_token(&user_id, "admin@example.com", &UserRole::Admin, secret)
            .expect("token should be issued");

        let claims = decode_access_token(&token, secret).expect("token should decode");
        assert_eq!(claims.sub, user_id.to_hex());
        assert_eq!(claims.email, "admin@example.com");
        assert_eq!(claims.role, "admin");
    }
}
