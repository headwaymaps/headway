pub mod atlas;
pub mod dmfr;
pub mod extents;
pub mod feed_config;
pub mod geohash;
pub mod geom;
pub mod measure;
pub mod onestop;
pub mod progress;
pub mod realtime;
pub mod zone;

pub use geo::{coord, Coord, Rect};
pub use geom::RectExt;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;
