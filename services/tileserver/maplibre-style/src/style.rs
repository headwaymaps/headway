//! The style document itself.

use crate::layer::{Background, Fill, FillExtrusion, Kind, Layer, LayerData, Line, Raster, Symbol};
use crate::source::{Source, SourceLayer, SourceRef};
use serde::Serialize;
use serde_json::{json, Map, Value as Json};
use std::collections::BTreeMap;

/// Writes whole numbers as integers, drops float noise, and sorts object keys, so regenerating
/// the style yields a minimal diff whether or not serde_json's `preserve_order` is enabled
/// elsewhere in the build.
fn canonical(value: &Json) -> Json {
    match value {
        Json::Number(number) => match number.as_f64().map(|number| (number * 1e6).round() / 1e6) {
            Some(number) if number.fract() == 0.0 && number.abs() < 1e15 => json!(number as i64),
            Some(number) if number.is_finite() => json!(number),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    light: Option<Light>,
    sources: BTreeMap<String, Source>,
    layers: Vec<LayerData>,
}

/// The light that shades 3D extrusions; MapLibre's defaults fill in the rest.
#[derive(Debug, Serialize)]
pub struct Light {
    pub intensity: f64,
}

/// Symbol settings shared by several layers, each of which starts from them.
pub struct SymbolLayerFactory<'a> {
    style: &'a mut Style,
    template: LayerData,
}

impl SymbolLayerFactory<'_> {
    /// Appends a symbol layer that starts from the shared settings.
    pub fn symbol(&mut self, source: &SourceLayer, id: impl Into<String>) -> Layer<'_, Symbol> {
        let data = LayerData::new::<Symbol>(id, Some(&source.source), Some(&source.source_layer))
            .with_template(&self.template);
        self.style.add(data)
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
            light: None,
            sources: BTreeMap::new(),
            layers: Vec::new(),
        }
    }

    /// Shares symbol settings, configured by `configure`, across several layers.
    pub fn symbol_factory(
        &mut self,
        configure: impl FnOnce(Layer<'_, Symbol>),
    ) -> SymbolLayerFactory<'_> {
        let mut template = LayerData::new::<Symbol>("", None, None);
        configure(Layer::new(&mut template));
        SymbolLayerFactory {
            style: self,
            template,
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

    pub fn light(&mut self, light: Light) {
        self.light = Some(light);
    }

    /// Registers a source and returns the handle layers are built from.
    pub fn source(&mut self, name: &str, source: Source) -> SourceRef {
        self.sources.insert(name.to_owned(), source);
        SourceRef::new(name)
    }

    /// Appends a fill layer sourcing features from `source`.
    pub fn fill(&mut self, source: &SourceLayer, id: impl Into<String>) -> Layer<'_, Fill> {
        self.add_sourced(source, id)
    }

    /// Appends a 3D extruded fill layer sourcing features from `source`.
    pub fn fill_extrusion(
        &mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> Layer<'_, FillExtrusion> {
        self.add_sourced(source, id)
    }

    /// Appends a background layer at this exact point in the style.
    pub fn background(&mut self, id: impl Into<String>) -> Layer<'_, Background> {
        self.add(LayerData::new::<Background>(id, None, None))
    }

    /// Appends a raster layer sourcing tiles directly from `source`.
    pub fn raster(&mut self, source: &SourceRef, id: impl Into<String>) -> Layer<'_, Raster> {
        self.add(LayerData::new::<Raster>(id, Some(source.name()), None))
    }

    /// Appends a line layer sourcing features from `source`.
    pub fn line(&mut self, source: &SourceLayer, id: impl Into<String>) -> Layer<'_, Line> {
        self.add_sourced(source, id)
    }

    /// Appends a symbol layer sourcing features from `source`.
    pub fn symbol(&mut self, source: &SourceLayer, id: impl Into<String>) -> Layer<'_, Symbol> {
        self.add_sourced(source, id)
    }

    fn add_sourced<K: Kind>(
        &mut self,
        source: &SourceLayer,
        id: impl Into<String>,
    ) -> Layer<'_, K> {
        self.add(LayerData::new::<K>(
            id,
            Some(&source.source),
            Some(&source.source_layer),
        ))
    }

    fn add<K: Kind>(&mut self, data: LayerData) -> Layer<'_, K> {
        let index = self.layers.len();
        self.layers.push(data);
        Layer::new(&mut self.layers[index])
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
