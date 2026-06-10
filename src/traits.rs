//! Abstraction layer — trait definitions that let real implementations be
//! swapped for mocks in tests (mirrors the reference's `traits/` + mock stores).
//!
//! The current app calls `queries/` functions against a concrete
//! `mongodb::Database` directly, which is fine at this size. As the data layer
//! grows, define store traits here (e.g. `UserStore`, `TaskStore`), implement
//! them for the real Mongo store and for an in-memory mock, and have `compute/`
//! depend on the trait object instead of the concrete database.
//!
//! Example shape (left commented until needed):
//!
//! ```ignore
//! #[async_trait::async_trait]
//! pub trait UserStore {
//!     async fn find_by_email(&self, email: &str) -> Result<Option<User>, ApiError>;
//!     async fn insert(&self, user: &User) -> Result<ObjectId, ApiError>;
//! }
//! ```
