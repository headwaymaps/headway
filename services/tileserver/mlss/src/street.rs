//! Street stacking: every casing has to be painted below every street surface,
//! otherwise a casing covers the street it neighbours at an intersection.

use crate::layer::{AnyLayer, Layer, Line};
use crate::property::Property;

/// Collects street layers and their casings, keeping the two tiers separate.
#[derive(Debug, Default)]
pub struct Streets {
    casings: Vec<Layer<Line>>,
    surfaces: Vec<Layer<Line>>,
}

impl Streets {
    pub fn new() -> Self {
        Streets::default()
    }

    /// Adds a street, plus a casing `pad` wider on each side that sinks to the casing tier.
    pub fn cased(
        &mut self,
        street: Layer<Line>,
        pad: f64,
        color: impl Into<Property>,
    ) -> &mut Self {
        self.casings.push(street.casing(pad, color));
        self.surfaces.push(street);
        self
    }

    /// Adds a street that has no casing, such as a path or a rail line.
    pub fn plain(&mut self, street: Layer<Line>) -> &mut Self {
        self.surfaces.push(street);
        self
    }

    /// Adds an already-built casing, for the rare one that isn't a widened copy.
    pub fn casing(&mut self, casing: Layer<Line>) -> &mut Self {
        self.casings.push(casing);
        self
    }
}

impl IntoIterator for Streets {
    type Item = AnyLayer;
    type IntoIter = std::vec::IntoIter<AnyLayer>;

    /// All casings first, then all surfaces, each in the order they were added.
    fn into_iter(self) -> Self::IntoIter {
        self.casings
            .into_iter()
            .chain(self.surfaces)
            .map(AnyLayer::from)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{zoom, Color, Source, Style};

    fn ids(streets: Streets) -> Vec<String> {
        streets
            .into_iter()
            .map(|layer| layer.id().to_owned())
            .collect()
    }

    #[test]
    fn casings_sink_below_every_surface() {
        let mut style = Style::new("test");
        let roads = style
            .source("t", Source::vector("/t"))
            .layer("transportation");
        let width = zoom([(12.0, 1.0), (20.0, 10.0)]);

        let mut streets = Streets::new();
        streets.plain(roads.line("path").width(width.clone()));
        streets.cased(
            roads.line("minor").width(width.clone()),
            1.0,
            Color::parse("#ccc"),
        );
        streets.cased(
            roads.line("motorway").width(width),
            1.0,
            Color::parse("#ccc"),
        );

        assert_eq!(
            ids(streets),
            [
                "minor_casing",
                "motorway_casing",
                "path",
                "minor",
                "motorway"
            ]
        );
    }

    #[test]
    fn a_casing_widens_every_stop_of_its_street() {
        let mut style = Style::new("test");
        let roads = style
            .source("t", Source::vector("/t"))
            .layer("transportation");
        let street = roads
            .line("minor")
            .width(zoom([(12.0, 0.0), (14.0, 2.0), (20.0, 10.0)]));

        let casing = AnyLayer::from(street.casing(1.5, Color::parse("#ccc"))).to_json();
        // The zero stop stays zero, so the casing appears with the street.
        assert_eq!(
            casing["paint"]["line-width"]["stops"],
            serde_json::json!([[12.0, 0.0], [14.0, 5.0], [20.0, 13.0]])
        );
    }
}
