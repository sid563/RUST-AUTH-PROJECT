//! THE router. Wired into the actix `App` from `main.rs` via
//! `.configure(create_connections::configure)`.
//!
//! Layout mirrors the reference service:
//!   * public routes (health, auth, dev) registered at the top level;
//!   * authenticated routes live inside a rate-limited `web::scope("")`, each
//!     `web::resource(...)` wrapped with `SessionMiddleware` (authn at the
//!     route boundary). Admin-only authz is a `require_admin` check inside the
//!     respective handlers.
//!
//! Middleware order (outer → inner): RateLimit (scope) → SessionMiddleware
//! (resource) → handler.

use actix_web::web;

use crate::web_server::auth::{dev_email_logs, login, seed_users, verify_2fa};
use crate::web_server::health_check;
use crate::web_server::middlewares::rate_limit::RateLimit;
use crate::web_server::middlewares::session::SessionMiddleware;
use crate::web_server::tasks::{assign_tasks, create_task, view_my_tasks};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // ---- public routes (no auth) ----
        .service(health_check::health)
        .service(seed_users::seed_users)
        .service(login::login)
        .service(verify_2fa::verify_2fa)
        .service(dev_email_logs::dev_email_logs_latest)
        // ---- authenticated routes: rate-limited scope + per-resource session auth ----
        .service(
            web::scope("")
                .wrap(RateLimit)
                .service(
                    web::resource("/tasks")
                        .route(web::post().to(create_task::create_task))
                        .wrap(SessionMiddleware),
                )
                .service(
                    web::resource("/tasks/assign")
                        .route(web::post().to(assign_tasks::assign_tasks))
                        .wrap(SessionMiddleware),
                )
                .service(
                    web::resource("/tasks/view-my-tasks")
                        .route(web::get().to(view_my_tasks::view_my_tasks))
                        .wrap(SessionMiddleware),
                ),
        );
}
