//! Redis connection factory.

/// Build a Redis client from the connection URL. The client is cheap to clone
/// and lazily opens multiplexed connections on first use.
pub fn connect_redis(url: &str) -> redis::Client {
    redis::Client::open(url).expect("invalid redis url")
}
