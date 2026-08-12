use crate::router_config::{RouterConfig, SkippedRealtime};
use crate::Result;

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Bumped when a field changes meaning, so readers refuse configurations they
/// would otherwise silently misinterpret.
pub const VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub version: u32,
    pub bounds: Bounds,
    pub feeds: Vec<ZoneFeed>,
}

impl Zone {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("reading zone from {}: {e}", path.display()))?;
        Self::parse(&contents).map_err(|e| format!("parsing {}: {e}", path.display()).into())
    }

    pub fn parse(contents: &str) -> Result<Self> {
        let zone: Self = serde_json::from_str(contents)?;
        if zone.version != VERSION {
            return Err(format!(
                "zone file is version {}, but this build only understands version {VERSION}",
                zone.version
            )
            .into());
        }
        Ok(zone)
    }

    pub fn router_config(&self) -> (RouterConfig, Vec<SkippedRealtime>) {
        RouterConfig::for_zone(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneFeed {
    pub feed_onestop_id: String,
    pub provider: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<ZoneAuth>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub realtime: Vec<ZoneRealtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneRealtime {
    pub feed_onestop_id: String,
    pub urls: RealtimeUrls,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<ZoneAuth>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RealtimeUrls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_updates: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_positions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneAuth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info_url: Option<String>,
    #[serde(default)]
    pub credential: String,
}
