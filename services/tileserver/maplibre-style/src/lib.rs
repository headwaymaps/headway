//! Builders for generating MapLibre style documents.
//!
//! ```
//! use maplibre_style::*;
//!
//! let mut style = Style::new("Example");
//! let tiles = style.source("openmaptiles", Source::vector("/tiles"));
//! let roads = tiles.layer("transportation");
//!
//! style
//!     .line(&roads, "motorway")
//!     .filter(Filter::class("motorway"))
//!     .rounded()
//!     .color(rgb!(255, 204, 136))
//!     .width(zoom!(base: 1.2; 7 => 1.0, 20 => 18.0));
//! ```

pub mod color;
pub mod expression;
pub mod filter;
pub mod headway;
pub mod layer;
pub mod open_map_tiles;
pub mod property;
pub mod source;
pub mod style;

pub use color::Color;
pub use expression::{Expression, Operator};
pub use filter::{ClassFilter, Filter};
pub use layer::{Background, Cap, Fill, Join, Kind, Layer, Line, Raster, Symbol};
pub use open_map_tiles::OpenMapTiles;
pub use property::{zoom, zoom_with_base, Property, Stops};
pub use source::{Source, SourceLayer, SourceRef};
pub use style::{Style, SymbolLayerFactory};
