use mongodb::bson::DateTime;
use mongodb::Database;

use crate::errors::ApiError;
use crate::models::{User, UserRole};
use crate::queries::users;
use crate::utils::security::hash_password;

/// The two demo accounts seeded for local validation.
pub const SEED_USERS: [(&str, &str, &str, UserRole); 2] = [
    ("Admin", "admin@example.com", "Admin@123", UserRole::Admin),
    (
        "James Bond",
        "jamesbond@example.com",
        "Bond@123",
        UserRole::Staff,
    ),
];

/// Idempotently seed demo users. Returns the emails newly inserted this call.
pub async fn seed_users(db: &Database) -> Result<Vec<String>, ApiError> {
    let mut inserted = Vec::new();

    for (full_name, email, raw_password, role) in SEED_USERS {
        if users::find_by_email(db, email).await?.is_some() {
            continue;
        }

        let password_hash = hash_password(raw_password)?;
        let now = DateTime::now();
        let user = User {
            id: None,
            full_name: full_name.to_string(),
            email: email.to_string(),
            password_hash,
            role,
            created_at: now,
            updated_at: now,
        };

        users::insert(db, &user).await?;
        inserted.push(email.to_string());
    }

    Ok(inserted)
}
