use futures::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::Database;

use crate::errors::ApiError;
use crate::models::Task;
use crate::utils::constants::TASKS_COLLECTION;

/// Insert a task and return its generated ObjectId.
pub async fn insert(db: &Database, task: &Task) -> Result<ObjectId, ApiError> {
    let tasks = db.collection::<Task>(TASKS_COLLECTION);
    let result = tasks.insert_one(task).await?;
    result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| ApiError::Internal("failed to read inserted task id".into()))
}

pub async fn find_by_ids(db: &Database, ids: &[ObjectId]) -> Result<Vec<Task>, ApiError> {
    let tasks = db.collection::<Task>(TASKS_COLLECTION);
    let cursor = tasks.find(doc! { "_id": { "$in": ids } }).await?;
    Ok(cursor.try_collect().await?)
}

pub async fn find_assigned_to(db: &Database, user_id: &ObjectId) -> Result<Vec<Task>, ApiError> {
    let tasks = db.collection::<Task>(TASKS_COLLECTION);
    let cursor = tasks.find(doc! { "assigned_to_id": user_id }).await?;
    Ok(cursor.try_collect().await?)
}

/// Assign the given tasks to a user (bulk `$set`).
pub async fn assign_to(
    db: &Database,
    ids: &[ObjectId],
    assignee_id: &ObjectId,
    updated_at: DateTime,
) -> Result<(), ApiError> {
    let tasks = db.collection::<Task>(TASKS_COLLECTION);
    tasks
        .update_many(
            doc! { "_id": { "$in": ids } },
            doc! { "$set": { "assigned_to_id": assignee_id, "updated_at": updated_at } },
        )
        .await?;
    Ok(())
}
