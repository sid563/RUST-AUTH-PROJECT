//! Domain data structures — pure data, no side effects.
//!
//! One file per domain entity. Common types are re-exported at the module
//! root so call sites can use `crate::models::User` regardless of which file
//! a struct lives in.

pub mod auth;
pub mod dtos;
pub mod session;
pub mod task;
pub mod user;

pub use auth::{EmailLog, LoginChallenge};
pub use session::{AuthUser, JwtClaims};
pub use task::{Task, TaskPriority, TaskStatus};
pub use user::{User, UserRole};
