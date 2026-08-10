pub mod api_keys;
pub mod dmfr;
pub mod extents;
pub mod feed_id;
pub mod geohash;
pub mod geom;
pub mod measure;
pub mod onestop;
pub mod prefilter;
pub mod realtime;

pub use geom::{Point, Rect};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;
