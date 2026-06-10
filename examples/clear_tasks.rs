//! Dev helper: wipe the `tasks` and `login_challenges` collections so the e2e
//! validation flow starts from a clean slate.
//!
//!   cargo run --example clear_tasks

use mongodb::bson::doc;
use mongodb::Client;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let db_name = std::env::var("MONGO_DB_NAME").unwrap_or_else(|_| "task_auth_db".to_string());

    let client = Client::with_uri_str(&uri).await.expect("connect failed");
    let db = client.database(&db_name);

    for coll in ["tasks", "login_challenges", "email_logs"] {
        let res = db
            .collection::<mongodb::bson::Document>(coll)
            .delete_many(doc! {})
            .await
            .expect("delete failed");
        println!("cleared {coll}: {} docs", res.deleted_count);
    }
}
