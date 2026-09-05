use crate::router_config::{RouterConfig, SkippedRealtime};
use crate::Result;

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Bumped when a field changes meaning, so readers refuse configurations they
/// would otherwise silently misinterpret.
pub const VERSION: u32 = 1;

/// `deny_unknown_fields` throughout, because the failure it prevents has
/// already happened: zones written before the updaters moved out of the
/// document still carried a `router_config` section, serde dropped it without
/// a word, and the Bay Area and Puget Sound deployed with no realtime at all.
/// A field this schema doesn't know is a document written for a different
/// reader, and `version` is how a real schema change announces itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
        // Version first, and on its own: with `deny_unknown_fields` a document
        // from a newer schema would otherwise fail on whichever field it added,
        // and "unknown field `foo`" is a much worse answer than "that's
        // version 2".
        #[derive(Deserialize)]
        struct Versioned {
            version: u32,
        }
        let versioned: Versioned = serde_json::from_str(contents)?;
        if versioned.version != VERSION {
            return Err(format!(
                "zone file is version {}, but this build only understands version {VERSION}",
                versioned.version
            )
            .into());
        }

        Ok(serde_json::from_str(contents)?)
    }

    pub fn router_config(&self) -> (RouterConfig, Vec<SkippedRealtime>) {
        RouterConfig::for_zone(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ZoneRealtime {
    pub feed_onestop_id: String,
    pub urls: RealtimeUrls,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<ZoneAuth>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealtimeUrls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_updates: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_positions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> serde_json::Value {
        serde_json::json!({
            "version": VERSION,
            "bounds": { "min_lon": -1.0, "min_lat": -1.0, "max_lon": 1.0, "max_lat": 1.0 },
            "feeds": [{
                "feed_onestop_id": "f-c23-kcm",
                "provider": "King County Metro",
                "url": "https://example.com/gtfs.zip",
            }],
        })
    }

    #[test]
    fn reads_a_zone_the_picker_wrote() {
        let zone = Zone::parse(&minimal().to_string()).unwrap();
        assert_eq!(zone.feeds.len(), 1);
        assert!(zone.feeds[0].realtime.is_empty());
    }

    /// The regression this schema is guarded against: a zone written before the
    /// updaters moved out of the document. Accepting it drops its realtime
    /// without saying so.
    #[test]
    fn a_zone_from_the_old_schema_is_refused_rather_than_read_past() {
        let mut old = minimal();
        old["router_config"] = serde_json::json!({ "updaters": [] });

        let err = Zone::parse(&old.to_string()).unwrap_err().to_string();
        assert!(err.contains("router_config"), "{err}");
    }

    #[test]
    fn a_stray_field_inside_a_feed_is_refused_too() {
        let mut zone = minimal();
        zone["feeds"][0]["realtiem"] = serde_json::json!([]);

        let err = Zone::parse(&zone.to_string()).unwrap_err().to_string();
        assert!(err.contains("realtiem"), "{err}");
    }

    /// A newer document should say so, rather than blaming whichever field it
    /// happens to have added.
    #[test]
    fn a_future_version_fails_on_the_version() {
        let mut future = minimal();
        future["version"] = serde_json::json!(VERSION + 1);
        future["something_new"] = serde_json::json!(true);

        let err = Zone::parse(&future.to_string()).unwrap_err().to_string();
        assert!(err.contains("version"), "{err}");
        assert!(!err.contains("something_new"), "{err}");
    }
}
