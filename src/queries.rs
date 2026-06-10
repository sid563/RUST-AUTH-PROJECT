//! Database access layer. Every function here only talks to MongoDB —
//! find/insert/update. No business logic, no HTTP, no caching.

pub mod auth;
pub mod tasks;
pub mod users;
