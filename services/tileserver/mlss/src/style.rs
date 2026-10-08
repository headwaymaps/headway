//! The style document itself.

use crate::layer::{AnyLayer, Background, Fill, Kind, Layer, Line, Raster, Symbol};
use crate::openmaptiles::OpenMapTiles;
use crate::source::{background, Source, SourceLayer, SourceRef};
use crate::{Cap, Filter, Join, Property};
use serde_json::{json, Map, Value as Json};

/// Zooms and widths are all f64 here, but `"minzoom": 8.0` is noise in a style file.
fn whole_numbers_as_integers(value: &Json) -> Json {
    match value {
        Json::Number(number) => match number.as_f64() {
            Some(number) if number.fract() == 0.0 && number.abs() < 1e15 => json!(number as i64),
            _ => value.clone(),
        },
        Json::Array(items) => Json::Array(items.iter().map(whole_numbers_as_integers).collect()),
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), whole_numbers_as_integers(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[derive(Debug)]
pub struct Style {
    id: Option<String>,
    name: String,
    sprite: Option<String>,
    glyphs: Option<String>,
    metadata: Map<String, Json>,
    sources: Vec<(String, Source)>,
    layers: Vec<AnyLayer>,
}

/// A rendering layer being configured directly on a style.
///
/// It is appended when the builder reaches the end of its statement, which
/// makes style-layer order the order in which `Style::{fill,line,symbol}` are
/// called.
pub struct AttachedLayer<'a, K: Kind> {
    style: &'a mut Style,
    layer: Option<Layer<K>>,
}

/// A source layer while it is adding rendering layers to a style.
pub struct LayerAdder<'a> {
    style: &'a mut Style,
    source: SourceLayer,
}

impl LayerAdder<'_> {
    pub fn fill(&mut self, id: impl Into<String>) -> AttachedLayer<'_, Fill> {
        self.style.fill(&self.source, id)
    }

    pub fn line(&mut self, id: impl Into<String>) -> AttachedLayer<'_, Line> {
        self.style.line(&self.source, id)
    }

    pub fn symbol(&mut self, id: impl Into<String>) -> AttachedLayer<'_, Symbol> {
        self.style.symbol(&self.source, id)
    }
}

impl<K: Kind> AttachedLayer<'_, K> {
    fn map(mut self, change: impl FnOnce(Layer<K>) -> Layer<K>) -> Self {
        self.layer = Some(change(
            self.layer.take().expect("attached layer is present"),
        ));
        self
    }

    pub fn minzoom(self, zoom: f64) -> Self {
        self.map(|layer| layer.minzoom(zoom))
    }

    pub fn maxzoom(self, zoom: f64) -> Self {
        self.map(|layer| layer.maxzoom(zoom))
    }

    pub fn filter(self, filter: Filter) -> Self {
        self.map(|layer| layer.filter(filter))
    }

    pub fn layout(self, name: &str, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.layout(name, value))
    }

    pub fn paint(self, name: &str, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.paint(name, value))
    }
}

impl AttachedLayer<'_, Fill> {
    pub fn color(self, color: impl Into<Property>) -> Self {
        self.map(|layer| layer.color(color))
    }

    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.map(|layer| layer.opacity(opacity))
    }

    pub fn outline_color(self, color: impl Into<Property>) -> Self {
        self.map(|layer| layer.outline_color(color))
    }

    pub fn antialias(self, antialias: bool) -> Self {
        self.map(|layer| layer.antialias(antialias))
    }

    pub fn pattern(self, pattern: impl Into<Property>) -> Self {
        self.map(|layer| layer.pattern(pattern))
    }
}

impl AttachedLayer<'_, Line> {
    pub fn color(self, color: impl Into<Property>) -> Self {
        self.map(|layer| layer.color(color))
    }

    pub fn width(self, width: impl Into<Property>) -> Self {
        self.map(|layer| layer.width(width))
    }

    pub fn opacity(self, opacity: impl Into<Property>) -> Self {
        self.map(|layer| layer.opacity(opacity))
    }

    pub fn dasharray(self, dashes: impl Into<Property>) -> Self {
        self.map(|layer| layer.dasharray(dashes))
    }

    pub fn gap_width(self, width: impl Into<Property>) -> Self {
        self.map(|layer| layer.gap_width(width))
    }

    pub fn cap(self, cap: Cap) -> Self {
        self.map(|layer| layer.cap(cap))
    }

    pub fn cap_round(self) -> Self {
        self.map(Layer::cap_round)
    }

    pub fn join(self, join: Join) -> Self {
        self.map(|layer| layer.join(join))
    }

    pub fn join_round(self) -> Self {
        self.map(Layer::join_round)
    }

    pub fn rounded(self) -> Self {
        self.map(Layer::rounded)
    }
}

impl AttachedLayer<'_, Symbol> {
    pub fn text_field(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.text_field(value))
    }

    pub fn text_color(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.text_color(value))
    }

    pub fn text_font<'a>(self, fonts: impl IntoIterator<Item = &'a str>) -> Self {
        self.map(|layer| layer.text_font(fonts))
    }

    pub fn text_size(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.text_size(value))
    }

    pub fn text_max_width(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.text_max_width(value))
    }

    pub fn text_offset(self, value: [f64; 2]) -> Self {
        self.map(|layer| layer.text_offset(value))
    }

    pub fn text_anchor(self, value: &str) -> Self {
        self.map(|layer| layer.text_anchor(value))
    }

    pub fn text_justify(self, value: &str) -> Self {
        self.map(|layer| layer.text_justify(value))
    }

    pub fn text_transform(self, value: &str) -> Self {
        self.map(|layer| layer.text_transform(value))
    }

    pub fn text_letter_spacing(self, value: f64) -> Self {
        self.map(|layer| layer.text_letter_spacing(value))
    }

    pub fn text_rotation_alignment(self, value: &str) -> Self {
        self.map(|layer| layer.text_rotation_alignment(value))
    }

    pub fn text_halo(self, color: impl Into<Property>, width: f64) -> Self {
        self.map(|layer| layer.text_halo(color, width))
    }

    pub fn text_halo_blur(self, blur: f64) -> Self {
        self.map(|layer| layer.text_halo_blur(blur))
    }

    pub fn icon_image(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.icon_image(value))
    }

    pub fn icon_size(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.icon_size(value))
    }

    pub fn icon_rotate(self, value: f64) -> Self {
        self.map(|layer| layer.icon_rotate(value))
    }

    pub fn icon_rotation_alignment(self, value: &str) -> Self {
        self.map(|layer| layer.icon_rotation_alignment(value))
    }

    pub fn icon_allow_overlap(self, value: bool) -> Self {
        self.map(|layer| layer.icon_allow_overlap(value))
    }

    pub fn icon_optional(self, value: bool) -> Self {
        self.map(|layer| layer.icon_optional(value))
    }

    pub fn placement(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.placement(value))
    }

    pub fn spacing(self, value: f64) -> Self {
        self.map(|layer| layer.spacing(value))
    }
}

impl AttachedLayer<'_, Raster> {
    pub fn opacity(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.opacity(value))
    }
}

impl AttachedLayer<'_, Background> {
    pub fn color(self, value: impl Into<Property>) -> Self {
        self.map(|layer| layer.color(value))
    }
}

impl<K: Kind> Drop for AttachedLayer<'_, K> {
    fn drop(&mut self) {
        if let Some(layer) = self.layer.take() {
            self.style.push(layer);
        }
    }
}

impl Style {
    pub fn new(name: impl Into<String>) -> Self {
        Style {
            id: None,
            name: name.into(),
            sprite: None,
            glyphs: None,
            metadata: Map::new(),
            sources: Vec::new(),
            layers: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn sprite(mut self, url: impl Into<String>) -> Self {
        self.sprite = Some(url.into());
        self
    }

    pub fn glyphs(mut self, url: impl Into<String>) -> Self {
        self.glyphs = Some(url.into());
        self
    }

    pub fn metadata(mut self, key: &str, value: impl Into<Json>) -> Self {
        self.metadata.insert(key.to_owned(), value.into());
        self
    }

    /// Registers a source and returns the handle layers are built from.
    pub fn source(&mut self, name: &str, source: Source) -> SourceRef {
        self.sources.push((name.to_owned(), source));
        SourceRef::new(name)
    }

    /// Adds a vector-tile source and returns a handle for its source layers.
    pub fn vector_source(&mut self, name: &str, url: impl Into<String>) -> SourceRef {
        self.source(name, Source::vector(url))
    }

    /// Adds a raster-tile source and returns a handle for drawing it.
    pub fn raster_source(&mut self, name: &str, url: impl Into<String>) -> SourceRef {
        self.source(name, Source::raster(url))
    }

    /// Adds Headway's OpenMapTiles source and returns its schema-aware handle.
    pub fn open_map_tiles_source(&mut self, url: impl Into<String>) -> OpenMapTiles {
        OpenMapTiles::new(self.source("openmaptiles", Source::vector(url).encoding("mlt")))
    }

    /// Appends a fill layer sourcing features from `source`.
    pub fn fill<'a>(
        &'a mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> AttachedLayer<'a, Fill> {
        AttachedLayer {
            style: self,
            layer: Some(source.fill(id)),
        }
    }

    /// Appends a background layer at this exact point in the style.
    pub fn background<'a>(&'a mut self, id: impl Into<String>) -> AttachedLayer<'a, Background> {
        AttachedLayer {
            style: self,
            layer: Some(background(id)),
        }
    }

    /// Appends a raster layer sourcing tiles directly from `source`.
    pub fn raster<'a>(
        &'a mut self,
        source: &SourceRef,
        id: impl Into<String>,
    ) -> AttachedLayer<'a, Raster> {
        AttachedLayer {
            style: self,
            layer: Some(source.raster(id)),
        }
    }

    /// Appends a line layer sourcing features from `source`.
    pub fn line<'a>(
        &'a mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> AttachedLayer<'a, Line> {
        AttachedLayer {
            style: self,
            layer: Some(source.line(id)),
        }
    }

    /// Appends a symbol layer sourcing features from `source`.
    pub fn symbol<'a>(
        &'a mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> AttachedLayer<'a, Symbol> {
        AttachedLayer {
            style: self,
            layer: Some(source.symbol(id)),
        }
    }

    /// Adds rendering layers sourced from `source` in closure declaration order.
    pub fn add_layers(&mut self, source: SourceLayer, build: impl FnOnce(&mut LayerAdder<'_>)) {
        build(&mut LayerAdder {
            style: self,
            source,
        });
    }

    pub fn push(&mut self, layer: impl Into<AnyLayer>) -> &mut Self {
        self.layers.push(layer.into());
        self
    }

    pub fn extend<L: Into<AnyLayer>>(&mut self, layers: impl IntoIterator<Item = L>) -> &mut Self {
        self.layers.extend(layers.into_iter().map(Into::into));
        self
    }

    pub fn layer_ids(&self) -> impl Iterator<Item = &str> {
        self.layers.iter().map(AnyLayer::id)
    }

    pub fn to_json(&self) -> Json {
        let mut map = Map::new();
        map.insert("version".into(), json!(8));
        map.insert("name".into(), json!(self.name));
        if !self.metadata.is_empty() {
            map.insert("metadata".into(), Json::Object(self.metadata.clone()));
        }
        let sources: Map<String, Json> = self
            .sources
            .iter()
            .map(|(name, source)| (name.clone(), source.to_json()))
            .collect();
        map.insert("sources".into(), Json::Object(sources));
        if let Some(sprite) = &self.sprite {
            map.insert("sprite".into(), json!(sprite));
        }
        if let Some(glyphs) = &self.glyphs {
            map.insert("glyphs".into(), json!(glyphs));
        }
        map.insert(
            "layers".into(),
            Json::Array(self.layers.iter().map(AnyLayer::to_json).collect()),
        );
        if let Some(id) = &self.id {
            map.insert("id".into(), json!(id));
        }
        whole_numbers_as_integers(&Json::Object(map))
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).expect("a style is always serializable")
    }

    /// Panics if two layers share an id, which maplibre rejects at load time.
    pub fn validate(&self) {
        let mut seen = std::collections::HashSet::new();
        for layer in &self.layers {
            assert!(
                seen.insert(layer.id()),
                "duplicate layer id: {}",
                layer.id()
            );
        }
    }
}
