mod applications;
mod compute;
mod config;
mod errors;
mod models;
mod queries;
mod request_validations;
mod traits;
mod utils;
mod web_server;

use std::{collections::HashMap, sync::Arc};

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use tokio::sync::RwLock;
use tracing_subscriber::{fmt, EnvFilter};

use applications::application_store::AppState;
use applications::{connect_mongo::connect_mongo, connect_redis::connect_redis};
use config::AppConfig;
use web_server::create_connections;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let cfg = AppConfig::from_env();

    // Infrastructure connections (see `applications/`).
    let db = connect_mongo(&cfg.mongo_uri, &cfg.mongo_db_name).await;
    let redis_client = connect_redis(&cfg.redis_url);

    let app_state = AppState {
        db,
        jwt_secret: cfg.jwt_secret.clone(),
        redis_client,
        dev_email_events: Arc::new(RwLock::new(HashMap::new())),
        rate_limit_per_second: cfg.rate_limit_per_second,
    };

    let bind_address = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("server listening on {bind_address}");

    HttpServer::new(move || {
        // Permissive CORS for local development (the UI client at :3000 talks
        // to this API at :8080). Tighten the allowed origin before any non-local
        // deployment.
        App::new()
            .wrap(Cors::permissive())
            .app_data(web::Data::new(app_state.clone()))
            .configure(create_connections::configure)
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
