//! Business logic. Compute functions orchestrate `queries/` (data) and
//! `utils/` (jwt, hashing, cache), and never touch HTTP request/response types.
//! `web_server/` handlers call into here.

pub mod auth;
pub mod authorization;
pub mod tasks;
