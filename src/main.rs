mod config;
mod jwt;
mod models;
mod routes;
mod security;
mod state;

use std::{collections::HashMap, sync::Arc};

use actix_web::{web, App, HttpServer};
use config::AppConfig;
use mongodb::Client;
use routes::{
    assign_tasks, create_task, dev_email_logs_latest, health, login, seed_users, verify_2fa,
    view_my_tasks,
};
use state::AppState;
use tokio::sync::RwLock;
use tracing_subscriber::{fmt, EnvFilter};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let cfg = AppConfig::from_env();

    let mongo_client = Client::with_uri_str(&cfg.mongo_uri)
        .await
        .expect("failed to connect to mongodb");
    let db = mongo_client.database(&cfg.mongo_db_name);

    let redis_client = redis::Client::open(cfg.redis_url.clone()).expect("invalid redis url");

    let app_state = AppState {
        db,
        jwt_secret: cfg.jwt_secret.clone(),
        redis_client,
        dev_email_events: Arc::new(RwLock::new(HashMap::new())),
    };

    let bind_address = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("server listening on {bind_address}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(health)
            .service(seed_users)
            .service(login)
            .service(verify_2fa)
            .service(dev_email_logs_latest)
            .service(create_task)
            .service(assign_tasks)
            .service(view_my_tasks)
    })
    .bind(bind_address)?
    .run()
    .await
}

fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,actix_web=info"));
    fmt().with_env_filter(env_filter).init();
}
