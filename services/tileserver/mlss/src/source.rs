//! Tile sources, and the handles used to attach layers to them.

use crate::layer::{Background, Fill, Layer, Line, Raster, Symbol};
use serde_json::{json, Map, Value as Json};

#[derive(Clone, Debug)]
pub struct Source {
    kind: &'static str,
    url: String,
    minzoom: Option<f64>,
    maxzoom: Option<f64>,
    attribution: Option<String>,
    encoding: Option<String>,
}

impl Source {
    pub fn vector(url: impl Into<String>) -> Self {
        Source {
            kind: "vector",
            url: url.into(),
            minzoom: None,
            maxzoom: None,
            attribution: None,
            encoding: None,
        }
    }

    pub fn raster(url: impl Into<String>) -> Self {
        Source {
            kind: "raster",
            ..Source::vector(url)
        }
    }

    pub fn minzoom(mut self, zoom: f64) -> Self {
        self.minzoom = Some(zoom);
        self
    }

    pub fn maxzoom(mut self, zoom: f64) -> Self {
        self.maxzoom = Some(zoom);
        self
    }

    pub fn attribution(mut self, attribution: impl Into<String>) -> Self {
        self.attribution = Some(attribution.into());
        self
    }

    /// The tile encoding, e.g. `mlt` for maplibre tiles.
    pub fn encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = Some(encoding.into());
        self
    }

    pub(crate) fn to_json(&self) -> Json {
        let mut map = Map::new();
        if let Some(maxzoom) = self.maxzoom {
            map.insert("maxzoom".into(), json!(maxzoom));
        }
        if let Some(minzoom) = self.minzoom {
            map.insert("minzoom".into(), json!(minzoom));
        }
        map.insert("type".into(), json!(self.kind));
        map.insert("url".into(), json!(self.url));
        if let Some(attribution) = &self.attribution {
            map.insert("attribution".into(), json!(attribution));
        }
        if let Some(encoding) = &self.encoding {
            map.insert("encoding".into(), json!(encoding));
        }
        Json::Object(map)
    }
}

/// A named source in a style; hand it out to build layers that draw from it.
#[derive(Clone, Debug)]
pub struct SourceRef {
    name: String,
}

impl SourceRef {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        SourceRef { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// One layer of a vector tile, e.g. `transportation`.
    pub fn layer(&self, source_layer: &str) -> SourceLayer {
        SourceLayer {
            source: self.name.clone(),
            source_layer: Some(source_layer.to_owned()),
        }
    }

    /// A raster source has no sub-layers, so it is drawn from directly.
    pub fn raster(&self, id: impl Into<String>) -> Layer<Raster> {
        Layer::new(id, Some(&self.name), None)
    }
}

/// A single vector-tile layer within a source: the thing features come from.
#[derive(Clone, Debug)]
pub struct SourceLayer {
    source: String,
    source_layer: Option<String>,
}

macro_rules! features {
    ($($method:ident => $kind:ty),*) => {
        impl SourceLayer {
            $(
                pub fn $method(&self, id: impl Into<String>) -> Layer<$kind> {
                    Layer::new(id, Some(&self.source), self.source_layer.as_deref())
                }
            )*
        }
    };
}

features!(line => Line, fill => Fill, symbol => Symbol);

/// The background isn't drawn from any source.
pub fn background(id: impl Into<String>) -> Layer<Background> {
    Layer::<Background>::new(id, None, None)
}
