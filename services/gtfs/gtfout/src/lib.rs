pub mod geom;
pub use geom::{Point, Rect};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;
