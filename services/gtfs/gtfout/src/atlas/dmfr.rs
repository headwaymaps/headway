//! Reading feed records out of a transitland-atlas clone.

use crate::Result;

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DmfrFile {
    #[serde(default)]
    feeds: Vec<RawFeed>,
    #[serde(default)]
    operators: Vec<Operator>,
}

/// Everything we read out of an atlas clone.
pub struct Catalog {
    /// Static feeds, each carrying the realtime feeds that update it.
    pub static_feeds: Vec<StaticFeed>,
    unsupported: Vec<UnsupportedFeed>,
}

impl Catalog {
    pub fn unsupported_spec(&self, id: &str) -> Option<&str> {
        self.unsupported
            .iter()
            .find(|feed| feed.id == id)
            .map(|feed| feed.spec.as_str())
    }
}

/// What every feed record has, whatever it publishes.
#[derive(Debug, Clone)]
pub struct Feed {
    pub id: String,

    pub operators: Vec<Operator>,

    pub authorization: Option<Authorization>,
}

/// A `gtfs` feed as the atlas lists it: a schedule to fetch, plus whatever
/// realtime rides along with it.
#[derive(Debug, Clone)]
pub struct StaticFeed {
    pub feed: Feed,

    /// Where to fetch the feed today. This is the agency's own endpoint.
    pub url: Option<String>,

    pub realtime: Vec<RealtimeFeed>,
}

impl StaticFeed {
    pub fn id(&self) -> &str {
        &self.feed.id
    }

    pub fn display_name(&self) -> String {
        self.feed.display_name()
    }

    pub fn authorization(&self) -> Option<&Authorization> {
        self.feed.authorization.as_ref()
    }
}

/// A `gtfs-rt` feed. It has no stops, so it has no extent and can never be
/// found by a spatial query - it reaches a zone only through [`StaticFeed`].
#[derive(Debug, Clone)]
pub struct RealtimeFeed {
    pub feed: Feed,
    pub alerts_url: Option<String>,
    pub trip_updates_url: Option<String>,
    pub vehicle_positions_url: Option<String>,
}

impl RealtimeFeed {
    pub fn id(&self) -> &str {
        &self.feed.id
    }

    pub fn authorization(&self) -> Option<&Authorization> {
        self.feed.authorization.as_ref()
    }
}

/// A record we don't build from, remembered by id alone.
#[derive(Debug, Clone)]
struct UnsupportedFeed {
    id: String,
    spec: String,
}

/// One record exactly as DMFR writes it, before `spec` has been read.
#[derive(Debug, Deserialize)]
struct RawFeed {
    id: String,

    spec: String,

    #[serde(default)]
    urls: RawUrls,

    #[serde(default)]
    operators: Vec<Operator>,

    #[serde(default)]
    authorization: Option<Authorization>,
}

#[derive(Debug, Default, Deserialize)]
struct RawUrls {
    #[serde(default)]
    static_current: Option<String>,
    #[serde(default)]
    realtime_alerts: Option<String>,
    #[serde(default)]
    realtime_trip_updates: Option<String>,
    #[serde(default)]
    realtime_vehicle_positions: Option<String>,
}

impl RawFeed {
    fn core(&self) -> Feed {
        Feed {
            id: self.id.clone(),
            operators: self.operators.clone(),
            authorization: self.authorization.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Operator {
    pub onestop_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub short_name: Option<String>,
    /// Feeds this operator draws data from. The only link DMFR records between
    /// a GTFS-RT feed and the static feed it updates - see [`super::realtime`].
    #[serde(default)]
    pub associated_feeds: Vec<AssociatedFeed>,
}

/// An entry in an operator's `associated_feeds`.
#[derive(Debug, Clone, Deserialize)]
pub struct AssociatedFeed {
    #[serde(default)]
    pub feed_onestop_id: Option<String>,
}

/// How to authenticate to `urls.static_current`.
#[derive(Debug, Clone, Deserialize)]
pub struct Authorization {
    /// `query_param`, `header`, `basic_auth`, or `replace_url`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub param_name: Option<String>,
    #[serde(default)]
    pub info_url: Option<String>,
}

impl Feed {
    /// A name to show a human curating the output.
    pub fn display_name(&self) -> String {
        let names: Vec<&str> = self
            .operators
            .iter()
            .filter_map(|op| {
                op.name
                    .as_deref()
                    .or(op.short_name.as_deref())
                    .filter(|name| !name.is_empty())
            })
            .collect();

        if names.is_empty() {
            self.id.clone()
        } else {
            names.join(", ")
        }
    }
}

/// Loads every feed and top-level operator from an atlas clone.
pub fn load_catalog(atlas_dir: &Path) -> Result<Catalog> {
    let feeds_dir = atlas_dir.join("feeds");
    if !feeds_dir.is_dir() {
        return Err(format!(
            "{} doesn't look like a transitland-atlas clone: no feeds/ directory",
            atlas_dir.display()
        )
        .into());
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&feeds_dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::result::Result<_, _>>()?;
    paths.sort();

    let mut records: Vec<RawFeed> = Vec::new();
    let mut operators = Vec::new();
    for path in paths {
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        let parsed: DmfrFile = serde_json::from_str(&contents)
            .map_err(|e| format!("parsing {}: {e}", path.display()))?;

        records.extend(parsed.feeds);
        operators.extend(parsed.operators);
    }

    Ok(catalog(records, operators))
}

/// Sorts parsed records by spec and joins realtime onto static.
fn catalog(records: Vec<RawFeed>, operators: Vec<Operator>) -> Catalog {
    let mut gtfs_records = Vec::new();
    let mut realtime = Vec::new();
    let mut unsupported = Vec::new();

    for raw in records {
        match raw.spec.as_str() {
            "gtfs" => {
                let feed = raw.core();
                gtfs_records.push((feed, raw.urls));
            }
            "gtfs-rt" => realtime.push(RealtimeFeed {
                feed: raw.core(),
                alerts_url: raw.urls.realtime_alerts,
                trip_updates_url: raw.urls.realtime_trip_updates,
                vehicle_positions_url: raw.urls.realtime_vehicle_positions,
            }),
            _ => unsupported.push(UnsupportedFeed {
                id: raw.id,
                spec: raw.spec,
            }),
        }
    }

    let associations = super::realtime::Associations::build(
        gtfs_records
            .iter()
            .map(|(feed, _)| feed)
            .chain(realtime.iter().map(|rt| &rt.feed)),
        &operators,
    );
    let mut by_static = super::realtime::realtime_by_static(
        gtfs_records.iter().map(|(feed, _)| feed.id.as_str()),
        &realtime,
        &associations,
    );

    let static_feeds = gtfs_records
        .into_iter()
        .map(|(feed, urls)| StaticFeed {
            realtime: by_static.remove(&feed.id).unwrap_or_default(),
            feed,
            url: urls.static_current,
        })
        .collect();

    unsupported.extend(realtime.into_iter().map(|rt| UnsupportedFeed {
        id: rt.feed.id,
        spec: "gtfs-rt".to_owned(),
    }));

    Catalog {
        static_feeds,
        unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "$schema": "https://dmfr.transit.land/json-schema/dmfr.schema-v0.6.0.json",
      "feeds": [
        {
          "id": "f-9q8y-sfmta",
          "spec": "gtfs",
          "urls": {
            "static_current": "http://api.511.org/transit/datafeeds?operator_id=SF",
            "static_historic": ["https://gtfs.sfmta.com/transitdata/google_transit.zip"]
          },
          "authorization": {
            "type": "query_param",
            "param_name": "api_key",
            "info_url": "https://511.org/open-data/token"
          },
          "tags": { "manual_import": "true" }
        },
        {
          "id": "f-smart~ca~us",
          "spec": "gtfs",
          "urls": { "static_current": "https://example.com/smart.zip" },
          "operators": [
            {
              "onestop_id": "o-9qc-smart",
              "name": "Sonoma-Marin Area Rail Transit",
              "short_name": "SMART",
              "tags": { "us_ntd_id": "90232" }
            }
          ]
        },
        {
          "id": "f-9q9-actransit~rt",
          "spec": "gtfs-rt",
          "urls": { "realtime_alerts": "https://example.com/alerts" },
          "operators": [{ "onestop_id": "o-9q9-actransit" }]
        },
        {
          "id": "f-9q9-shared",
          "spec": "gtfs",
          "urls": { "static_current": "https://example.com/shared.zip" },
          "operators": [
            { "onestop_id": "o-9q9-actransit", "name": "AC Transit" },
            { "onestop_id": "o-9q9-wheels", "short_name": "Wheels" }
          ]
        }
      ]
    }"#;

    fn sample() -> Catalog {
        let parsed: DmfrFile = serde_json::from_str(SAMPLE).unwrap();
        catalog(parsed.feeds, parsed.operators)
    }

    #[test]
    fn parses_feeds() {
        let static_feeds = sample().static_feeds;
        assert_eq!(static_feeds.len(), 3);
        assert_eq!(static_feeds[0].id(), "f-9q8y-sfmta");
        assert_eq!(
            static_feeds[0].url.as_deref(),
            Some("http://api.511.org/transit/datafeeds?operator_id=SF")
        );
    }

    #[test]
    fn a_spec_we_dont_handle_is_remembered_but_not_a_feed() {
        let records =
            vec![
                serde_json::from_str::<RawFeed>(r#"{"id":"f-x-bikes","spec":"gbfs","urls":{}}"#)
                    .unwrap(),
            ];
        let catalog = catalog(records, vec![]);

        assert!(catalog.static_feeds.is_empty());
        assert_eq!(catalog.unsupported_spec("f-x-bikes"), Some("gbfs"));
        assert_eq!(sample().unsupported_spec("f-9q8y-sfmta"), None);
        assert_eq!(sample().unsupported_spec("f-nonesuch"), None);
    }

    #[test]
    fn realtime_is_joined_onto_the_static_feed_it_updates() {
        let static_feeds = sample().static_feeds;
        let shared = static_feeds
            .iter()
            .find(|f| f.id() == "f-9q9-shared")
            .unwrap();
        assert_eq!(shared.realtime.len(), 1);
        assert_eq!(shared.realtime[0].id(), "f-9q9-actransit~rt");
        assert_eq!(
            shared.realtime[0].alerts_url.as_deref(),
            Some("https://example.com/alerts")
        );

        let sfmta = static_feeds
            .iter()
            .find(|f| f.id() == "f-9q8y-sfmta")
            .unwrap();
        assert!(sfmta.realtime.is_empty());
    }

    #[test]
    fn parses_authorization() {
        let static_feeds = sample().static_feeds;
        let auth = static_feeds[0].authorization().unwrap();
        assert_eq!(auth.kind, "query_param");
        assert_eq!(auth.param_name.as_deref(), Some("api_key"));
        assert!(static_feeds[1].authorization().is_none());
    }

    #[test]
    fn display_name_prefers_the_operator() {
        let static_feeds = sample().static_feeds;
        assert_eq!(
            static_feeds[1].display_name(),
            "Sonoma-Marin Area Rail Transit"
        );
        assert_eq!(static_feeds[0].display_name(), "f-9q8y-sfmta");
    }

    #[test]
    fn display_name_lists_every_operator() {
        assert_eq!(
            sample().static_feeds[2].display_name(),
            "AC Transit, Wheels"
        );
    }
}
