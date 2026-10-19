//! Road classes, each drawn cased at every grade from a single definition.

use crate::*;

/// Where a road runs relative to the ground.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Grade {
    Tunnel,
    Surface,
    Bridge,
}

impl Grade {
    fn prefix(self) -> &'static str {
        match self {
            Grade::Tunnel => "tunnel",
            Grade::Surface => "road",
            Grade::Bridge => "bridge",
        }
    }

    fn filter(self) -> Filter {
        match self {
            Grade::Tunnel => super::tunnel(),
            Grade::Surface => super::surface(),
            Grade::Bridge => super::bridge(),
        }
    }

    /// Bridges end squarely at their abutments.
    fn cap(self) -> Cap {
        match self {
            Grade::Bridge => Cap::Butt,
            Grade::Tunnel | Grade::Surface => Cap::Round,
        }
    }

    /// Tunnel casings are dashed, like an outline seen through the ground, which needs square
    /// ends.
    fn style_casing<'a>(self, casing: Layer<'a, Line>) -> Layer<'a, Line> {
        match self {
            Grade::Tunnel => casing.cap(Cap::Butt).dasharray(vec![0.5, 0.25]),
            Grade::Surface | Grade::Bridge => casing.cap(self.cap()),
        }
    }

    /// Tunnels are washed out, as though seen through the ground.
    fn color(self, surface: &Property<Color>) -> Property<Color> {
        match self {
            Grade::Tunnel => surface.map(|color| color.mix(&hsl!(0, 0%, 100%), 0.5)),
            Grade::Surface | Grade::Bridge => surface.clone(),
        }
    }
}

/// The outline drawn beneath every road's fill.
enum Casing {
    From { minzoom: f64 },
    None,
}

/// A road or railway class as it looks at the surface.
pub(super) struct Road {
    name: &'static str,
    filter: Filter,
    minzoom: f64,
    casing: Casing,
    hatching: Option<Property<Color>>,
    color: Option<Property<Color>>,
    width: Option<Property<f64>>,
}

impl Road {
    /// A road drawn for features whose class is `name`, cased from z13.
    pub fn new(name: &'static str) -> Self {
        Road {
            name,
            filter: Filter::class(name),
            minzoom: 0.0,
            casing: Casing::From { minzoom: 13.0 },
            hatching: None,
            color: None,
            width: None,
        }
    }

    pub fn color(mut self, color: impl Into<Property<Color>>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn width(mut self, width: impl Into<Property<f64>>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    pub fn minzoom(mut self, zoom: f64) -> Self {
        self.minzoom = zoom;
        self
    }

    pub fn casing_minzoom(mut self, zoom: f64) -> Self {
        self.casing = Casing::From { minzoom: zoom };
        self
    }

    pub fn no_casing(mut self) -> Self {
        self.casing = Casing::None;
        self
    }

    /// Draws hatching across the line, like railroad ties.
    pub fn hatching(mut self, color: impl Into<Property<Color>>) -> Self {
        self.hatching = Some(color.into());
        self
    }

    fn fill_color(&self) -> &Property<Color> {
        self.color
            .as_ref()
            .unwrap_or_else(|| panic!("road {} needs a color", self.name))
    }

    fn fill_width(&self) -> &Property<f64> {
        self.width
            .as_ref()
            .unwrap_or_else(|| panic!("road {} needs a width", self.name))
    }

    fn casing_color(&self, grade: Grade) -> Property<Color> {
        grade.color(&self.fill_color().map(|color| color.darken(0.12)))
    }

    fn casing_width(&self) -> Property<f64> {
        self.fill_width().map(|width| 1.1 * width + 1.0)
    }
}

/// Road and railway classes, drawn bottom to top in the order they're listed.
pub(super) struct Roads<'a> {
    pub source: &'a SourceLayer,
    pub roads: Vec<Road>,
}

impl Roads<'_> {
    /// Adds every road's casing at `grade`, then every road's fill above them.
    pub fn add_layers(&self, style: &mut Style, grade: Grade) {
        let prefix = grade.prefix();
        for road in &self.roads {
            let Casing::From { minzoom } = road.casing else {
                continue;
            };
            let casing = style
                .line(self.source, format!("{prefix}_{}_casing", road.name))
                .minzoom(minzoom)
                .filter(grade.filter().and(road.filter.clone()))
                .join_round()
                .color(road.casing_color(grade))
                .width(road.casing_width());
            grade.style_casing(casing);
        }
        for road in &self.roads {
            style
                .line(self.source, format!("{prefix}_{}", road.name))
                .minzoom(road.minzoom)
                .filter(grade.filter().and(road.filter.clone()))
                .cap(grade.cap())
                .join_round()
                .color(grade.color(road.fill_color()))
                .width(road.fill_width().clone());
            if let Some(hatching) = &road.hatching {
                style
                    .line(self.source, format!("{prefix}_{}_hatching", road.name))
                    .filter(grade.filter().and(road.filter.clone()))
                    .color(grade.color(hatching))
                    .dasharray(vec![0.2, 4.0])
                    .width(zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0));
            }
        }
    }
}
