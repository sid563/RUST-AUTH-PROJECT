//! Queries for 2FA login challenges and email-log audit records.

use mongodb::bson::{doc, oid::ObjectId, Bson, DateTime};
use mongodb::Database;

use crate::errors::ApiError;
use crate::models::{EmailLog, LoginChallenge};
use crate::utils::constants::{EMAIL_LOGS_COLLECTION, LOGIN_CHALLENGES_COLLECTION};

pub async fn insert_challenge(
    db: &Database,
    challenge: &LoginChallenge,
) -> Result<ObjectId, ApiError> {
    let coll = db.collection::<LoginChallenge>(LOGIN_CHALLENGES_COLLECTION);
    let result = coll.insert_one(challenge).await?;
    result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| ApiError::Internal("failed to read challenge id".into()))
}

pub async fn find_challenge_by_id(
    db: &Database,
    id: &ObjectId,
) -> Result<Option<LoginChallenge>, ApiError> {
    let coll = db.collection::<LoginChallenge>(LOGIN_CHALLENGES_COLLECTION);
    Ok(coll.find_one(doc! { "_id": id }).await?)
}

/// Atomically mark a challenge as used. The `used_at: null` filter guarantees
/// one-time use even under concurrent verification attempts.
pub async fn mark_challenge_used(
    db: &Database,
    id: &ObjectId,
    used_at: DateTime,
) -> Result<(), ApiError> {
    let coll = db.collection::<LoginChallenge>(LOGIN_CHALLENGES_COLLECTION);
    coll.update_one(
        doc! { "_id": id, "used_at": Bson::Null },
        doc! { "$set": { "used_at": used_at } },
    )
    .await?;
    Ok(())
}

pub async fn insert_email_log(db: &Database, log: &EmailLog) -> Result<(), ApiError> {
    let coll = db.collection::<EmailLog>(EMAIL_LOGS_COLLECTION);
    coll.insert_one(log).await?;
    Ok(())
}
