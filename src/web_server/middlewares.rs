//! HTTP middlewares (auth, rate limiting, etc.), wrapped at the scope level in
//! `create_connections`, mirroring the reference's middleware layer.

pub mod rate_limit;
pub mod session;
