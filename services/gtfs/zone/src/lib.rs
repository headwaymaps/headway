pub mod api_keys;
pub mod feed_id;
pub mod router_config;
pub mod zone;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;
