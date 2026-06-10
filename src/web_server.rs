//! HTTP layer. Handlers are thin: extract request data, run validation, call
//! `compute/`, and render the response. No business logic lives here.
//!
//! `create_connections` wires every route; `main.rs` calls it via `.configure`.

pub mod auth;
pub mod create_connections;
pub mod health_check;
pub mod middlewares;
pub mod tasks;
