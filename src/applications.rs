//! Infrastructure connections and the shared application store.
//!
//! One file per external service (Mongo, Redis). `application_store` holds the
//! handles that get cloned into every request via actix `web::Data`.

pub mod application_store;
pub mod connect_mongo;
pub mod connect_redis;
