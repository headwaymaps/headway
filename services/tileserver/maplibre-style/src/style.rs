//! The style document itself.

use crate::layer::{AnyLayer, Background, Fill, Kind, Layer, Line, Raster, Symbol};
use crate::source::{background, Source, SourceLayer, SourceRef};
use crate::{Cap, Color, Filter, Join, Property};
use serde::Serialize;
use serde_json::{json, Map, Value as Json};
use std::collections::BTreeMap;

/// Writes whole numbers as integers and sorts object keys, so regenerating the style yields a
/// minimal diff whether or not serde_json's `preserve_order` is enabled elsewhere in the build.
fn canonical(value: &Json) -> Json {
    match value {
        Json::Number(number) => match number.as_f64() {
            Some(number) if number.fract() == 0.0 && number.abs() < 1e15 => json!(number as i64),
            _ => value.clone(),
        },
        Json::Array(items) => Json::Array(items.iter().map(canonical).collect()),
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        other => other.clone(),
    }
}

#[derive(Debug, Serialize)]
pub struct Style {
    version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sprite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    glyphs: Option<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    metadata: Map<String, Json>,
    sources: BTreeMap<String, Source>,
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

/// A reusable set of symbol properties that appends configured layers to a style.
pub struct SymbolLayerFactory<'a> {
    style: &'a mut Style,
    template: Layer<Symbol>,
}

/// A reusable set of line properties for one source layer.
#[derive(Clone)]
pub struct LineFactory {
    source: SourceLayer,
    template: Layer<Line>,
}

impl LineFactory {
    fn map(mut self, change: impl FnOnce(Layer<Line>) -> Layer<Line>) -> Self {
        self.template = change(self.template);
        self
    }

    /// Starts a configured line layer from this template.
    pub fn line<'a>(&self, style: &'a mut Style, id: impl Into<String>) -> AttachedLayer<'a, Line> {
        AttachedLayer {
            style,
            layer: Some(self.source.line(id).with_template(&self.template)),
        }
    }

    pub fn color(self, value: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.color(value))
    }

    pub fn width(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.width(value))
    }

    pub fn opacity(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.opacity(value))
    }

    pub fn dasharray(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.dasharray(value))
    }

    pub fn rounded(self) -> Self {
        self.map(Layer::rounded)
    }
}

impl SymbolLayerFactory<'_> {
    fn map(mut self, change: impl FnOnce(Layer<Symbol>) -> Layer<Symbol>) -> Self {
        self.template = change(self.template);
        self
    }

    /// Starts a configured symbol layer from this template.
    pub fn symbol(
        &mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> AttachedLayer<'_, Symbol> {
        AttachedLayer {
            style: &mut *self.style,
            layer: Some(source.symbol(id).with_template(&self.template)),
        }
    }

    pub fn text_field(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.text_field(value))
    }

    pub fn text_font<'a>(self, fonts: impl IntoIterator<Item = &'a str>) -> Self {
        self.map(|layer| layer.text_font(fonts))
    }

    pub fn text_size(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.text_size(value))
    }

    pub fn text_max_width(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.text_max_width(value))
    }

    pub fn text_offset(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.text_offset(value))
    }

    pub fn text_anchor(self, value: &str) -> Self {
        self.map(|layer| layer.text_anchor(value))
    }

    pub fn text_color(self, value: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.text_color(value))
    }

    pub fn text_halo(self, color: impl Into<Property<Color>>, width: f64) -> Self {
        self.map(|layer| layer.text_halo(color, width))
    }

    pub fn text_halo_blur(self, blur: f64) -> Self {
        self.map(|layer| layer.text_halo_blur(blur))
    }

    pub fn icon_image(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.icon_image(value))
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

    pub fn paint(self, name: &str, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.paint(name, value))
    }
}

impl AttachedLayer<'_, Fill> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.color(color))
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.opacity(opacity))
    }

    pub fn outline_color(self, color: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.outline_color(color))
    }

    pub fn antialias(self, antialias: bool) -> Self {
        self.map(|layer| layer.antialias(antialias))
    }

    pub fn pattern(self, pattern: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.pattern(pattern))
    }
}

impl AttachedLayer<'_, Line> {
    pub fn color(self, color: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.color(color))
    }

    pub fn width(self, width: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.width(width))
    }

    pub fn opacity(self, opacity: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.opacity(opacity))
    }

    pub fn dasharray(self, dashes: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.dasharray(dashes))
    }

    pub fn gap_width(self, width: impl Into<Property<f64>>) -> Self {
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
    pub fn text_field(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.text_field(value))
    }

    pub fn text_color(self, value: impl Into<Property<Color>>) -> Self {
        self.map(|layer| layer.text_color(value))
    }

    pub fn text_font<'a>(self, fonts: impl IntoIterator<Item = &'a str>) -> Self {
        self.map(|layer| layer.text_font(fonts))
    }

    pub fn text_size(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.text_size(value))
    }

    pub fn text_max_width(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.text_max_width(value))
    }

    pub fn text_offset(self, value: impl Into<Property<Json>>) -> Self {
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

    pub fn text_letter_spacing(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.text_letter_spacing(value))
    }

    pub fn text_rotation_alignment(self, value: &str) -> Self {
        self.map(|layer| layer.text_rotation_alignment(value))
    }

    pub fn text_halo(self, color: impl Into<Property<Color>>, width: f64) -> Self {
        self.map(|layer| layer.text_halo(color, width))
    }

    pub fn text_halo_blur(self, blur: f64) -> Self {
        self.map(|layer| layer.text_halo_blur(blur))
    }

    pub fn icon_image(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.icon_image(value))
    }

    pub fn icon_size(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.icon_size(value))
    }

    pub fn icon_rotate(self, value: impl Into<Property<f64>>) -> Self {
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

    pub fn placement(self, value: impl Into<Property<Json>>) -> Self {
        self.map(|layer| layer.placement(value))
    }

    pub fn spacing(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.spacing(value))
    }
}

impl AttachedLayer<'_, Raster> {
    pub fn opacity(self, value: impl Into<Property<f64>>) -> Self {
        self.map(|layer| layer.opacity(value))
    }
}

impl AttachedLayer<'_, Background> {
    pub fn color(self, value: impl Into<Property<Color>>) -> Self {
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
            version: 8,
            id: None,
            name: name.into(),
            sprite: None,
            glyphs: None,
            metadata: Map::new(),
            sources: BTreeMap::new(),
            layers: Vec::new(),
        }
    }

    /// Starts a reusable symbol-layer template that appends to this style.
    pub fn symbol_factory(&mut self) -> SymbolLayerFactory<'_> {
        SymbolLayerFactory {
            style: self,
            template: Layer::new("", None, None),
        }
    }

    /// Starts a reusable line-layer template for `source`.
    pub fn line_factory(&self, source: SourceLayer) -> LineFactory {
        LineFactory {
            source,
            template: Layer::new("", None, None),
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
        self.sources.insert(name.to_owned(), source);
        SourceRef::new(name)
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

    fn push(&mut self, layer: impl Into<AnyLayer>) {
        self.layers.push(layer.into());
    }

    pub fn to_json(&self) -> Json {
        canonical(&serde_json::to_value(self).expect("a style is always serializable"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serializes_with_serde() {
        let mut style = Style::new("Test");
        let source = style.source("tiles", Source::raster("/tiles"));

        assert_eq!(source.name(), "tiles");
        assert_eq!(
            style.to_json()["sources"]["tiles"],
            json!({ "type": "raster", "url": "/tiles" })
        );
    }
}
