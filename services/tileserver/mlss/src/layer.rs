//! Layers, typed by the kind of geometry they paint.

use crate::filter::Filter;
use crate::property::Property;
use serde_json::{json, Map, Value as Json};
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
    Line => "line",
    Symbol => "symbol"
);

#[derive(Clone, Debug)]
struct Body {
    id: String,
    kind: &'static str,
    source: Option<String>,
    source_layer: Option<String>,
    minzoom: Option<f64>,
    maxzoom: Option<f64>,
    filter: Option<Filter>,
    layout: BTreeMap<String, Property>,
    paint: BTreeMap<String, Property>,
}

/// A style layer. The type parameter fixes which paint and layout properties apply.
#[derive(Clone, Debug)]
pub struct Layer<K> {
    body: Body,
    kind: PhantomData<K>,
}

/// A layer whose kind has been erased so it can sit in the style's layer list.
#[derive(Clone, Debug)]
pub struct AnyLayer(Body);

impl<K: Kind> Layer<K> {
    pub(crate) fn new(
        id: impl Into<String>,
        source: Option<&str>,
        source_layer: Option<&str>,
    ) -> Self {
        Layer {
            body: Body {
                id: id.into(),
                kind: K::TYPE,
                source: source.map(str::to_owned),
                source_layer: source_layer.map(str::to_owned),
                minzoom: None,
                maxzoom: None,
                filter: None,
                layout: BTreeMap::new(),
                paint: BTreeMap::new(),
            },
            kind: PhantomData,
        }
    }

    pub fn id(&self) -> &str {
        &self.body.id
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.body.id = id.into();
        self
    }

    pub fn minzoom(mut self, zoom: f64) -> Self {
        self.body.minzoom = Some(zoom);
        self
    }

    pub fn maxzoom(mut self, zoom: f64) -> Self {
        self.body.maxzoom = Some(zoom);
        self
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.body.filter = Some(filter);
        self
    }

    /// Adds a clause to the filter, or sets it if there isn't one yet.
    pub fn and_filter(mut self, filter: Filter) -> Self {
        self.body.filter = Some(match self.body.filter.take() {
            Some(existing) => existing.and(filter),
            None => filter,
        });
        self
    }

    pub fn hidden(self) -> Self {
        self.layout("visibility", "none")
    }

    /// Sets a layout property by its spec name.
    pub fn layout(mut self, name: &str, value: impl Into<Property>) -> Self {
        self.body.layout.insert(name.to_owned(), value.into());
        self
    }

    /// Sets a paint property by its spec name.
    pub fn paint(mut self, name: &str, value: impl Into<Property>) -> Self {
        self.body.paint.insert(name.to_owned(), value.into());
        self
    }

    /// Applies layout and paint settings from a reusable layer template.
    pub(crate) fn with_template(mut self, template: &Self) -> Self {
        self.body.layout.extend(template.body.layout.clone());
        self.body.paint.extend(template.body.paint.clone());
        self
    }

    fn get_paint(&self, name: &str) -> Option<&Property> {
        self.body.paint.get(name)
    }
}

impl<K: Kind> Layer<K> {
    /// Ends a layer's builder chain by adding it to a style.
    pub fn push_to(self, style: &mut crate::Style) {
        style.push(self);
    }
}

impl<K: Kind> From<Layer<K>> for AnyLayer {
    fn from(layer: Layer<K>) -> AnyLayer {
        AnyLayer(layer.body)
    }
}

impl AnyLayer {
    pub fn id(&self) -> &str {
        &self.0.id
    }

    pub fn to_json(&self) -> Json {
        let body = &self.0;
        let mut map = Map::new();
        map.insert("id".into(), json!(body.id));
        map.insert("type".into(), json!(body.kind));
        if let Some(source) = &body.source {
            map.insert("source".into(), json!(source));
        }
        if let Some(source_layer) = &body.source_layer {
            map.insert("source-layer".into(), json!(source_layer));
        }
        if let Some(minzoom) = body.minzoom {
            map.insert("minzoom".into(), json!(minzoom));
        }
        if let Some(maxzoom) = body.maxzoom {
            map.insert("maxzoom".into(), json!(maxzoom));
        }
        if let Some(filter) = &body.filter {
            map.insert("filter".into(), filter.to_json());
        }
        let section = |properties: &BTreeMap<String, Property>| {
            properties
                .iter()
                .map(|(name, value)| (name.clone(), value.to_json()))
                .collect::<Map<String, Json>>()
        };
        if !body.layout.is_empty() {
            map.insert("layout".into(), Json::Object(section(&body.layout)));
        }
        if !body.paint.is_empty() {
            map.insert("paint".into(), Json::Object(section(&body.paint)));
        }
        Json::Object(map)
    }
}

/// How a line's ends and corners are drawn.
#[derive(Clone, Copy, Debug)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug)]
pub enum Join {
    Bevel,
    Round,
    Miter,
}

impl Layer<Line> {
    pub fn color(self, color: impl Into<Property>) -> Self {
        self.paint("line-color", color)
    }

    pub fn width(self, width: impl Into<Property>) -> Self {
        self.paint("line-width", width)
    }

    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.paint("line-opacity", opacity)
    }

    pub fn gap_width(self, width: impl Into<Property>) -> Self {
        self.paint("line-gap-width", width)
    }

    pub fn offset(self, offset: impl Into<Property>) -> Self {
        self.paint("line-offset", offset)
    }

    pub fn blur(self, blur: impl Into<Property>) -> Self {
        self.paint("line-blur", blur)
    }

    pub fn dasharray(self, dashes: impl Into<Property>) -> Self {
        self.paint("line-dasharray", dashes)
    }

    pub fn cap(self, cap: Cap) -> Self {
        self.layout(
            "line-cap",
            match cap {
                Cap::Butt => "butt",
                Cap::Round => "round",
                Cap::Square => "square",
            },
        )
    }

    /// Uses round line caps.
    pub fn cap_round(self) -> Self {
        self.cap(Cap::Round)
    }

    pub fn join(self, join: Join) -> Self {
        self.layout(
            "line-join",
            match join {
                Join::Bevel => "bevel",
                Join::Round => "round",
                Join::Miter => "miter",
            },
        )
    }

    /// Uses round line joins.
    pub fn join_round(self) -> Self {
        self.join(Join::Round)
    }

    /// Rounds both the caps and the joins, which is what most road layers want.
    pub fn rounded(self) -> Self {
        self.cap_round().join_round()
    }

    /// The wider line that sits underneath this one, `pad` wider on each side.
    ///
    /// Zoom-varying widths are scaled stop by stop, so the casing tracks the line
    /// as it grows.
    pub fn casing(&self, pad: f64, color: impl Into<Property>) -> Self {
        let width = self
            .get_paint("line-width")
            .expect("a line needs a width before it can be cased")
            .map_numbers(|width| if width == 0.0 { 0.0 } else { width + pad * 2.0 });
        let mut casing = self.clone().with_id(format!("{}_casing", self.body.id));
        casing.body.paint.remove("line-dasharray");
        casing.width(width).color(color)
    }

    /// This line and its casing, bottom first, ready to push onto a style.
    pub fn with_casing(self, pad: f64, color: impl Into<Property>) -> [Layer<Line>; 2] {
        let casing = self.casing(pad, color);
        [casing, self]
    }
}

impl Layer<Fill> {
    pub fn color(self, color: impl Into<Property>) -> Self {
        self.paint("fill-color", color)
    }

    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.paint("fill-opacity", opacity)
    }

    pub fn outline_color(self, color: impl Into<Property>) -> Self {
        self.paint("fill-outline-color", color)
    }

    pub fn antialias(self, antialias: bool) -> Self {
        self.paint("fill-antialias", antialias)
    }

    pub fn pattern(self, pattern: impl Into<Property>) -> Self {
        self.paint("fill-pattern", pattern)
    }
}

impl Layer<Background> {
    pub fn color(self, color: impl Into<Property>) -> Self {
        self.paint("background-color", color)
    }

    pub fn pattern(self, pattern: impl Into<Property>) -> Self {
        self.paint("background-pattern", pattern)
    }

    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.paint("background-opacity", opacity)
    }
}

impl Layer<Raster> {
    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.paint("raster-opacity", opacity)
    }
}

impl Layer<Symbol> {
    pub fn text_field(self, field: impl Into<Property>) -> Self {
        self.layout("text-field", field)
    }

    pub fn text_font<'a>(self, fonts: impl IntoIterator<Item = &'a str>) -> Self {
        self.layout("text-font", fonts.into_iter().collect::<Vec<_>>())
    }

    pub fn text_size(self, size: impl Into<Property>) -> Self {
        self.layout("text-size", size)
    }

    pub fn text_max_width(self, width: impl Into<Property>) -> Self {
        self.layout("text-max-width", width)
    }

    pub fn text_offset(self, offset: [f64; 2]) -> Self {
        self.layout("text-offset", offset)
    }

    pub fn text_anchor(self, anchor: &str) -> Self {
        self.layout("text-anchor", anchor)
    }

    pub fn text_justify(self, justify: &str) -> Self {
        self.layout("text-justify", justify)
    }

    pub fn text_transform(self, transform: &str) -> Self {
        self.layout("text-transform", transform)
    }

    pub fn text_letter_spacing(self, spacing: f64) -> Self {
        self.layout("text-letter-spacing", spacing)
    }

    pub fn text_rotation_alignment(self, alignment: &str) -> Self {
        self.layout("text-rotation-alignment", alignment)
    }

    pub fn text_color(self, color: impl Into<Property>) -> Self {
        self.paint("text-color", color)
    }

    /// The halo that keeps a label legible over busy ground.
    pub fn text_halo(self, color: impl Into<Property>, width: f64) -> Self {
        self.paint("text-halo-color", color)
            .paint("text-halo-width", width)
    }

    pub fn text_halo_blur(self, blur: f64) -> Self {
        self.paint("text-halo-blur", blur)
    }

    pub fn icon_image(self, image: impl Into<Property>) -> Self {
        self.layout("icon-image", image)
    }

    pub fn icon_size(self, size: impl Into<Property>) -> Self {
        self.layout("icon-size", size)
    }

    pub fn icon_offset(self, offset: [f64; 2]) -> Self {
        self.layout("icon-offset", offset)
    }

    pub fn icon_rotate(self, degrees: f64) -> Self {
        self.layout("icon-rotate", degrees)
    }

    pub fn icon_rotation_alignment(self, alignment: &str) -> Self {
        self.layout("icon-rotation-alignment", alignment)
    }

    pub fn icon_allow_overlap(self, allow: bool) -> Self {
        self.layout("icon-allow-overlap", allow)
    }

    pub fn icon_optional(self, optional: bool) -> Self {
        self.layout("icon-optional", optional)
    }

    pub fn placement(self, placement: impl Into<Property>) -> Self {
        self.layout("symbol-placement", placement)
    }

    pub fn spacing(self, spacing: f64) -> Self {
        self.layout("symbol-spacing", spacing)
    }
}
