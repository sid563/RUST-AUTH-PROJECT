use mongodb::bson::{doc, oid::ObjectId};
use mongodb::Database;

use crate::errors::ApiError;
use crate::models::User;
use crate::utils::constants::USERS_COLLECTION;

pub async fn find_by_email(db: &Database, email: &str) -> Result<Option<User>, ApiError> {
    let users = db.collection::<User>(USERS_COLLECTION);
    Ok(users.find_one(doc! { "email": email.to_lowercase() }).await?)
}

pub async fn find_by_id(db: &Database, id: &ObjectId) -> Result<Option<User>, ApiError> {
    let users = db.collection::<User>(USERS_COLLECTION);
    Ok(users.find_one(doc! { "_id": id }).await?)
}

/// Insert a user and return its generated ObjectId.
pub async fn insert(db: &Database, user: &User) -> Result<ObjectId, ApiError> {
    let users = db.collection::<User>(USERS_COLLECTION);
    let result = users.insert_one(user).await?;
    result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| ApiError::Internal("failed to read inserted user id".into()))
}
