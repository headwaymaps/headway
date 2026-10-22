//! Layers, typed by the kind of geometry they paint.

use crate::color::Color;
use crate::filter::Filter;
use crate::property::Property;
use serde::Serialize;
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub trait Kind {
    const TYPE: &'static str;
}

macro_rules! kinds {
    ($($name:ident => $type:literal),*) => {
        $(
            #[derive(Clone, Copy, Debug)]
            pub struct $name;
            impl Kind for $name {
                const TYPE: &'static str = $type;
            }
        )*
    };
}

kinds!(
    Background => "background",
    Raster => "raster",
    Fill => "fill",
    FillExtrusion => "fill-extrusion",
    Line => "line",
    Symbol => "symbol"
);

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LayerData {
    id: String,
    #[serde(rename = "type")]
    _type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(rename = "source-layer", skip_serializing_if = "Option::is_none")]
    source_layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minzoom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxzoom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<Filter>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    layout: BTreeMap<String, Property<Json>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    paint: BTreeMap<String, Property<Json>>,
}

impl LayerData {
    pub(crate) fn new<K: Kind>(
        id: impl Into<String>,
        source: Option<&str>,
        source_layer: Option<&str>,
    ) -> Self {
        LayerData {
            id: id.into(),
            _type: K::TYPE,
            source: source.map(str::to_owned),
            source_layer: source_layer.map(str::to_owned),
            minzoom: None,
            maxzoom: None,
            filter: None,
            layout: BTreeMap::new(),
            paint: BTreeMap::new(),
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// Starts from the layout and paint settings of `template`.
    pub(crate) fn with_template(mut self, template: &LayerData) -> Self {
        self.layout.extend(template.layout.clone());
        self.paint.extend(template.paint.clone());
        self
    }
}

/// A style layer, configured in place. The type parameter fixes which paint and layout
/// properties apply.
pub struct Layer<'a, K> {
    data: &'a mut LayerData,
    _kind: PhantomData<K>,
}

impl<'a, K: Kind> Layer<'a, K> {
    pub(crate) fn new(data: &'a mut LayerData) -> Self {
        Layer {
            data,
            _kind: PhantomData,
        }
    }

    pub fn id(&self) -> &str {
        &self.data.id
    }

    pub fn minzoom(self, zoom: f64) -> Self {
        self.data.minzoom = Some(zoom);
        self
    }

    pub fn maxzoom(self, zoom: f64) -> Self {
        self.data.maxzoom = Some(zoom);
        self
    }

    pub fn filter(self, filter: Filter) -> Self {
        self.data.filter = Some(filter);
        self
    }

    /// Adds a clause to the filter, or sets it if there isn't one yet.
    pub fn and_filter(self, filter: Filter) -> Self {
        self.data.filter = Some(match self.data.filter.take() {
            Some(existing) => existing.and(filter),
            None => filter,
        });
        self
    }

    pub fn hidden(self) -> Self {
        self.layout("visibility", "none")
    }

    fn layout<T: Serialize>(self, name: &str, value: impl Into<Property<T>>) -> Self {
        let value: Property<T> = value.into();
        self.data.layout.insert(name.to_owned(), value.into_json());
        self
    }

    fn paint<T: Serialize>(self, name: &str, value: impl Into<Property<T>>) -> Self {
        let value: Property<T> = value.into();
        self.data.paint.insert(name.to_owned(), value.into_json());
        self
    }
}

/// How a line's ends are drawn.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cap {
    Butt,
    Round,
    Square,
}

/// How a line's corners are drawn.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Join {
    Bevel,
    Round,
    Miter,
}

/// Where a symbol sits relative to its feature.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    Point,
    Line,
    LineCenter,
}

/// Which part of a label sits on its anchor point.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    Center,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Justify {
    Auto,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transform {
    None,
    Uppercase,
    Lowercase,
}

/// What a label or icon rotates with.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    Map,
    Viewport,
    Auto,
}

impl Layer<'_, Line> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("line-color", color)
    }

    pub fn width(self, width: impl Into<Property<f64>>) -> Self {
        self.paint("line-width", width)
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.paint("line-opacity", opacity)
    }

    pub fn gap_width(self, width: impl Into<Property<f64>>) -> Self {
        self.paint("line-gap-width", width)
    }

    pub fn offset(self, offset: impl Into<Property<f64>>) -> Self {
        self.paint("line-offset", offset)
    }

    pub fn blur(self, blur: impl Into<Property<f64>>) -> Self {
        self.paint("line-blur", blur)
    }

    pub fn dasharray(self, dashes: impl Into<Property<Vec<f64>>>) -> Self {
        self.paint("line-dasharray", dashes)
    }

    pub fn cap(self, cap: Cap) -> Self {
        self.layout("line-cap", cap)
    }

    /// Uses round line caps.
    pub fn cap_round(self) -> Self {
        self.cap(Cap::Round)
    }

    /// Uses the renderer's default line cap.
    pub fn cap_default(self) -> Self {
        self.data.layout.remove("line-cap");
        self
    }

    pub fn join(self, join: Join) -> Self {
        self.layout("line-join", join)
    }

    /// Uses round line joins.
    pub fn join_round(self) -> Self {
        self.join(Join::Round)
    }

    /// Rounds both the caps and the joins, which is what most road layers want.
    pub fn rounded(self) -> Self {
        self.cap_round().join_round()
    }
}

impl Layer<'_, Fill> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("fill-color", color)
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.paint("fill-opacity", opacity)
    }

    pub fn outline_color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("fill-outline-color", color)
    }

    pub fn antialias(self, antialias: bool) -> Self {
        self.paint("fill-antialias", antialias)
    }

    pub fn pattern(self, pattern: impl Into<Property<String>>) -> Self {
        self.paint("fill-pattern", pattern)
    }
}

impl Layer<'_, FillExtrusion> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("fill-extrusion-color", color)
    }

    pub fn height(self, height: impl Into<Property<f64>>) -> Self {
        self.paint("fill-extrusion-height", height)
    }

    pub fn base(self, base: impl Into<Property<f64>>) -> Self {
        self.paint("fill-extrusion-base", base)
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.paint("fill-extrusion-opacity", opacity)
    }

    pub fn vertical_gradient(self, enabled: bool) -> Self {
        self.paint("fill-extrusion-vertical-gradient", enabled)
    }

    pub fn rounded_corner_distance(self, meters: f64) -> Self {
        self.layout("fill-extrusion-rounded-corner-distance", meters)
    }
}

impl Layer<'_, Background> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("background-color", color)
    }

    pub fn pattern(self, pattern: impl Into<Property<String>>) -> Self {
        self.paint("background-pattern", pattern)
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.paint("background-opacity", opacity)
    }
}

impl Layer<'_, Raster> {
    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.paint("raster-opacity", opacity)
    }
}

macro_rules! opacity_fades {
    ($($kind:ty => $property:literal),* $(,)?) => {
        $(
            impl Layer<'_, $kind> {
                pub fn fade_in(self, zoom: f64) -> Self {
                    self
                        .minzoom(zoom)
                        .opacity(crate::zoom!(zoom => 0.0, zoom + 0.5 => 1.0))
                }

                pub fn fade_out(self, zoom: f64) -> Self {
                    let fade = crate::zoom!(zoom => 1.0, zoom + 0.5 => 0.0);
                    match self.data.paint.get_mut($property) {
                        Some(Property::Stops(stops)) => stops.append(fade.map(|&opacity| opacity.into())),
                        _ => {
                            self.data.paint.insert($property.to_owned(), Property::from(fade).into_json());
                        }
                    }
                    self.maxzoom(zoom + 0.5)
                }
            }
        )*
    };
}

opacity_fades!(
    Line => "line-opacity",
    Fill => "fill-opacity",
    Background => "background-opacity",
    Raster => "raster-opacity",
);

impl Layer<'_, Symbol> {
    pub fn text_field(self, field: impl Into<Property<String>>) -> Self {
        self.layout("text-field", field)
    }

    pub fn text_font(self, font: impl Into<String>) -> Self {
        self.layout("text-font", vec![font.into()])
    }

    pub fn text_fonts<'a>(self, fonts: impl IntoIterator<Item = &'a str>) -> Self {
        self.layout(
            "text-font",
            fonts.into_iter().map(str::to_owned).collect::<Vec<_>>(),
        )
    }

    pub fn text_size(self, size: impl Into<Property<f64>>) -> Self {
        self.layout("text-size", size)
    }

    pub fn text_max_width(self, width: impl Into<Property<f64>>) -> Self {
        self.layout("text-max-width", width)
    }

    pub fn text_offset(self, offset: impl Into<Property<[f64; 2]>>) -> Self {
        self.layout("text-offset", offset)
    }

    pub fn text_anchor(self, anchor: Anchor) -> Self {
        self.layout("text-anchor", anchor)
    }

    pub fn text_justify(self, justify: Justify) -> Self {
        self.layout("text-justify", justify)
    }

    pub fn text_transform(self, transform: Transform) -> Self {
        self.layout("text-transform", transform)
    }

    pub fn text_letter_spacing(self, spacing: impl Into<Property<f64>>) -> Self {
        self.layout("text-letter-spacing", spacing)
    }

    pub fn text_rotation_alignment(self, alignment: Alignment) -> Self {
        self.layout("text-rotation-alignment", alignment)
    }

    pub fn text_color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("text-color", color)
    }

    /// The halo that keeps a label legible over busy ground.
    pub fn text_halo(
        self,
        color: impl Into<Property<Color>>,
        width: impl Into<Property<f64>>,
    ) -> Self {
        self.text_halo_color(color).text_halo_width(width)
    }

    pub fn text_halo_color(self, color: impl Into<Property<Color>>) -> Self {
        self.paint("text-halo-color", color)
    }

    pub fn text_halo_width(self, width: impl Into<Property<f64>>) -> Self {
        self.paint("text-halo-width", width)
    }

    pub fn text_halo_blur(self, blur: impl Into<Property<f64>>) -> Self {
        self.paint("text-halo-blur", blur)
    }

    pub fn icon_image(self, image: impl Into<Property<String>>) -> Self {
        self.layout("icon-image", image)
    }

    pub fn icon_size(self, size: impl Into<Property<f64>>) -> Self {
        self.layout("icon-size", size)
    }

    pub fn icon_offset(self, offset: [f64; 2]) -> Self {
        self.layout("icon-offset", offset)
    }

    pub fn icon_rotate(self, degrees: impl Into<Property<f64>>) -> Self {
        self.layout("icon-rotate", degrees)
    }

    pub fn icon_rotation_alignment(self, alignment: Alignment) -> Self {
        self.layout("icon-rotation-alignment", alignment)
    }

    pub fn icon_allow_overlap(self, allow: bool) -> Self {
        self.layout("icon-allow-overlap", allow)
    }

    pub fn icon_optional(self, optional: bool) -> Self {
        self.layout("icon-optional", optional)
    }

    pub fn placement(self, placement: impl Into<Property<Placement>>) -> Self {
        self.layout("symbol-placement", placement)
    }

    pub fn spacing(self, spacing: impl Into<Property<f64>>) -> Self {
        self.layout("symbol-spacing", spacing)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Source, Style};
    use serde_json::json;

    #[test]
    fn opacity_fades_can_be_chained() {
        let mut style = Style::new("Test");
        let roads = style
            .source("tiles", Source::vector("/tiles"))
            .layer("roads");
        style.line(&roads, "test").fade_in(13.0).fade_out(18.0);
        let layer = &style.to_json()["layers"][0];
        assert_eq!(layer["minzoom"], json!(13));
        assert_eq!(layer["maxzoom"], json!(18.5));
        assert_eq!(
            layer["paint"]["line-opacity"],
            json!({"stops": [[13, 0], [13.5, 1], [18, 1], [18.5, 0]]})
        );
    }
}
