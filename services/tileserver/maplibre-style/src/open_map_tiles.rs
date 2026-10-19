//! The named source layers defined by the OpenMapTiles schema.

use crate::{SourceLayer, SourceRef};

/// An OpenMapTiles vector source.
///
/// The wrapper makes its stable, schema-defined source-layer names available as
/// methods instead of repeating string literals throughout a style.
macro_rules! source_layers {
    ($($method:ident => $name:literal),* $(,)?) => {
        #[derive(Clone, Debug)]
        pub struct OpenMapTiles {
            $($method: SourceLayer,)*
        }

        impl OpenMapTiles {
            pub fn new(source: SourceRef) -> Self {
                Self {
                    $($method: source.layer($name),)*
                }
            }

            $(
                pub fn $method(&self) -> &SourceLayer {
                    &self.$method
                }
            )*
        }
    };
}

source_layers!(
    aeroway => "aeroway",
    boundary => "boundary",
    building => "building",
    landcover => "landcover",
    landuse => "landuse",
    park => "park",
    place => "place",
    poi => "poi",
    transportation => "transportation",
    transportation_name => "transportation_name",
    water => "water",
    water_name => "water_name",
    waterway => "waterway",
);
