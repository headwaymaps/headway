//! Road classes, each drawn cased at every grade from a single definition.

use crate::*;

/// Where a road runs relative to the ground.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Grade {
    /// Road features below grade.
    Tunnel,
    /// Road features at grade.
    Surface,
    /// Road features carried over another feature.
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

    pub fn filter(self) -> Filter {
        match self {
            Grade::Tunnel => Filter::eq("brunnel", "tunnel"),
            Grade::Surface => Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Grade::Bridge => Filter::eq("brunnel", "bridge"),
        }
    }

    /// Tunnel casings are dashed, like an outline seen through the ground, which needs square
    /// ends. Bridge casings end squarely at their abutments.
    fn style_casing<'a>(self, casing: Layer<'a, Line>) -> Layer<'a, Line> {
        match self {
            Grade::Tunnel => casing.cap(Cap::Butt).dasharray(vec![0.5, 0.25]),
            Grade::Bridge => casing.cap(Cap::Butt),
            Grade::Surface => casing.cap(Cap::Round),
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

/// How wide a line is drawn.
pub(super) enum Width {
    Pixels(Property<f64>),
    /// Cartographic `(zoom, pixels)` stops while zoomed out, then true to `meters` from
    /// [`Width::TRUE_TO_SCALE_ZOOM`].
    Meters {
        zoomed_out: Vec<(f64, f64)>,
        meters: Expression,
    },
}

impl Width {
    const TRUE_TO_SCALE_ZOOM: f64 = 16.0;
    const MAX_ZOOM: f64 = 24.0;

    /// Mercator stretches with latitude, which a style can't see, so scale is exact here and
    /// off by up to a fifth between the 33rd and 55th parallels.
    const REFERENCE_LATITUDE: f64 = 45.0;

    pub fn meters(zoomed_out: impl Into<Vec<(f64, f64)>>, meters: impl Into<Expression>) -> Self {
        let zoomed_out = zoomed_out.into();
        assert!(zoomed_out
            .iter()
            .all(|(zoom, _)| *zoom < Self::TRUE_TO_SCALE_ZOOM));
        Width::Meters {
            zoomed_out,
            meters: meters.into(),
        }
    }

    pub fn pixels_per_meter(zoom: f64) -> f64 {
        const EARTH_CIRCUMFERENCE: f64 = 40_075_016.686;
        const TILE_SIZE: f64 = 512.0;
        TILE_SIZE * zoom.exp2()
            / (EARTH_CIRCUMFERENCE * Self::REFERENCE_LATITUDE.to_radians().cos())
    }

    pub fn property(&self) -> Property<f64> {
        self.padded(|width| width, 0.0)
    }

    /// Widens zoomed-out stops with `pad_zoomed_out`, and true-to-scale ones by `pixels`.
    fn padded(&self, pad_zoomed_out: impl Fn(f64) -> f64, pixels: f64) -> Property<f64> {
        match self {
            Width::Pixels(width) => width.map(|width| pad_zoomed_out(*width)),
            Width::Meters { zoomed_out, meters } => {
                let mut operands = vec![expr!(["exponential", 2.0]), expr!(["zoom"])];
                for (zoom, width) in zoomed_out {
                    operands.push((*zoom).into());
                    operands.push(pad_zoomed_out(*width).into());
                }
                for zoom in [Self::TRUE_TO_SCALE_ZOOM, Self::MAX_ZOOM] {
                    let meters = meters.clone();
                    let scaled = expr!(["*", meters, (Self::pixels_per_meter(zoom))]);
                    operands.push(zoom.into());
                    operands.push(if pixels == 0.0 {
                        scaled
                    } else {
                        expr!(["+", scaled, pixels])
                    });
                }
                Expression::op(Operator::Interpolate, operands).into()
            }
        }
    }
}

impl From<Stops<f64>> for Width {
    fn from(stops: Stops<f64>) -> Self {
        Width::Pixels(stops.into())
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
    width: Option<Width>,
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

    pub fn width(mut self, width: impl Into<Width>) -> Self {
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

    fn has_surface(&self) -> bool {
        matches!(self.width, Some(Width::Meters { .. }))
    }

    /// Its color, blending into [`street_color`] as streets become true to scale and surfaced.
    fn fill_color(&self) -> Property<Color> {
        let color = self
            .color
            .as_ref()
            .unwrap_or_else(|| panic!("road {} needs a color", self.name));
        match color {
            Property::Stops(stops) if self.has_surface() => as_street(stops).into(),
            _ => color.clone(),
        }
    }

    fn fill_width(&self) -> &Width {
        self.width
            .as_ref()
            .unwrap_or_else(|| panic!("road {} needs a width", self.name))
    }

    fn casing_color(&self, grade: Grade) -> Property<Color> {
        grade.color(&self.fill_color().map(|color| color.darken(0.12)))
    }

    fn casing_width(&self) -> Property<f64> {
        self.fill_width().padded(|width| 1.1 * width + 1.0, 2.0)
    }
}

/// Every street's color once streets are true to scale.
fn street_color() -> Color {
    hsl!(197, 4%, 64%)
}

/// `color`, blending into [`street_color`] over the two zooms before streets are true to scale.
pub(super) fn as_street(color: &Stops<Color>) -> Stops<Color> {
    color.blend_into(
        Width::TRUE_TO_SCALE_ZOOM - 2.0,
        Width::TRUE_TO_SCALE_ZOOM,
        street_color(),
    )
}

/// Where streets switch from lines to the surfaces they cover, once lines are true to scale.
pub(super) const SURFACE_ZOOM: f64 = Width::TRUE_TO_SCALE_ZOOM - 0.5;

/// Road and railway classes, drawn bottom to top in the order they're listed.
pub(super) struct Roads<'a> {
    pub source: &'a SourceLayer,
    /// The ground each grade's streets with a width in meters cover, drawn in place of their lines from
    /// [`SURFACE_ZOOM`].
    pub surfaces: SourceLayer,
    pub roads: Vec<Road>,
}

impl Roads<'_> {
    /// A road's features at `grade`, but for roundabouts with a `radius`, which are drawn as
    /// circles instead.
    fn filter(road: &Road, grade: Grade) -> Filter {
        Filter::linestring()
            .and(grade.filter())
            .and(road.filter.clone())
            .and_not_has("radius")
    }

    /// Adds every road's casing at `grade`, then every road's fill above them.
    pub fn add_layers(&self, style: &mut Style, grade: Grade) {
        self.add_layers_with(style, grade, |_| {});
    }

    /// Adds every road's layers, with `beneath_fills` between the casings and the fills.
    pub fn add_layers_with(
        &self,
        style: &mut Style,
        grade: Grade,
        beneath_fills: impl FnOnce(&mut Style),
    ) {
        let prefix = grade.prefix();
        for road in &self.roads {
            let Casing::From { minzoom } = road.casing else {
                continue;
            };
            let casing = style
                .line(self.source, format!("{prefix}_{}_casing", road.name))
                .minzoom(minzoom)
                .filter(Self::filter(road, grade))
                .join_round()
                .color(road.casing_color(grade))
                .width(road.casing_width());
            let casing = grade.style_casing(casing);
            if road.has_surface() {
                casing.fade_out(SURFACE_ZOOM);
            }
        }
        beneath_fills(style);
        if self.roads.iter().any(Road::has_surface) {
            style
                .fill(&self.surfaces, format!("{prefix}_surface"))
                .fade_in(SURFACE_ZOOM)
                .filter(grade.filter())
                .color(grade.color(&street_color().into()));
        }
        for road in &self.roads {
            let fill = style
                .line(self.source, format!("{prefix}_{}", road.name))
                .minzoom(road.minzoom)
                .filter(Self::filter(road, grade))
                .cap(Cap::Round)
                .join_round()
                .color(grade.color(&road.fill_color()))
                .width(road.fill_width().property());
            if road.has_surface() {
                fill.fade_out(SURFACE_ZOOM);
            }
            if let Some(hatching) = &road.hatching {
                style
                    .line(self.source, format!("{prefix}_{}_hatching", road.name))
                    .filter(Self::filter(road, grade))
                    .color(grade.color(hatching))
                    .dasharray(vec![0.2, 4.0])
                    .width(zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0));
            }
        }
    }
}
