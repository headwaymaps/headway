//! Tile sources, and the handles used to attach layers to them.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Source {
    #[serde(rename = "type")]
    _type: &'static str,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    minzoom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxzoom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<Encoding>,
}

impl Source {
    pub fn vector(url: impl Into<String>) -> Self {
        Source {
            _type: "vector",
            url: url.into(),
            minzoom: None,
            maxzoom: None,
            attribution: None,
            encoding: None,
        }
    }

    pub fn raster(url: impl Into<String>) -> Self {
        Source {
            _type: "raster",
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

    pub fn encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = Some(encoding);
        self
    }
}

/// How a vector source's tiles are encoded.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    /// Mapbox Vector Tiles.
    Mvt,
    /// MapLibre Tiles.
    Mlt,
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
            source_layer: source_layer.to_owned(),
        }
    }
}

/// A single vector-tile layer within a source: the thing features come from.
#[derive(Clone, Debug)]
pub struct SourceLayer {
    pub(crate) source: String,
    pub(crate) source_layer: String,
}
