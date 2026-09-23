//! The style document itself.

use crate::layer::AnyLayer;
use crate::source::{Source, SourceRef};
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
    /// A parsed style document kept intact when MLSS is used as a compatibility
    /// wrapper around a canonical style.  This is useful for styles containing
    /// large, externally maintained expressions (such as route shields) that
    /// do not yet have a dedicated typed builder.
    raw: Option<Json>,
    id: Option<String>,
    name: String,
    sprite: Option<String>,
    glyphs: Option<String>,
    metadata: Map<String, Json>,
    sources: Vec<(String, Source)>,
    layers: Vec<AnyLayer>,
}

impl Style {
    pub fn new(name: impl Into<String>) -> Self {
        Style {
            raw: None,
            id: None,
            name: name.into(),
            sprite: None,
            glyphs: None,
            metadata: Map::new(),
            sources: Vec::new(),
            layers: Vec::new(),
        }
    }

    /// Loads an existing MapLibre style document without changing its meaning
    /// or serialization shape.
    pub fn from_json(input: &str) -> serde_json::Result<Self> {
        Ok(Style {
            raw: Some(serde_json::from_str(input)?),
            id: None,
            name: String::new(),
            sprite: None,
            glyphs: None,
            metadata: Map::new(),
            sources: Vec::new(),
            layers: Vec::new(),
        })
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
        if let Some(raw) = &self.raw {
            return raw.clone();
        }
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
        if let Some(raw) = &self.raw {
            let mut seen = std::collections::HashSet::new();
            for layer in raw["layers"].as_array().expect("a style has layers") {
                let id = layer["id"].as_str().expect("a layer has an id");
                assert!(seen.insert(id), "duplicate layer id: {id}");
            }
            return;
        }
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
