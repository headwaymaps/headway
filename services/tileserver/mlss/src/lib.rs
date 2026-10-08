#![recursion_limit = "512"]

//! Builders for generating MapLibre style documents.
//!
//! ```
//! use mlss::*;
//!
//! let mut style = Style::new("Example");
//! let tiles = style.source("openmaptiles", Source::vector("/tiles"));
//! let roads = tiles.layer("transportation");
//!
//! let mut streets = Streets::new();
//! streets.cased(
//!     roads.line("motorway").color(Color::parse("#fc8")).width(zoom!(base 1.2; 7 => 1.0, 20 => 18.0)),
//!     1.0,
//!     Color::parse("#fc8").darken(0.15),
//! );
//! style.extend(streets);
//! ```

pub mod color;
pub mod expression;
pub mod filter;
pub mod headway;
pub mod layer;
pub mod openmaptiles;
pub mod property;
pub mod source;
pub mod street;
pub mod style;

pub use color::Color;
pub use expression::{Expression, Operator};
pub use filter::{ClassFilter, Filter, Geometry};
pub use layer::{AnyLayer, Cap, Join, Layer};
pub use openmaptiles::OpenMapTiles;
pub use property::{stops, Property, Stops};
pub use source::{background, Source, SourceLayer, SourceRef};
pub use street::Streets;
pub use style::{Style, SymbolFactory};
