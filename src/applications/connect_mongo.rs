//! MongoDB connection factory.

use mongodb::{Client, Database};

/// Connect to MongoDB and return a handle to the configured database.
/// Panics on failure — a missing database at startup is unrecoverable.
pub async fn connect_mongo(uri: &str, db_name: &str) -> Database {
    let client = Client::with_uri_str(uri)
        .await
        .expect("failed to connect to mongodb");
    client.database(db_name)
}
