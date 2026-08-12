//! Reading feed records out of a transitland-atlas clone.
//!
//! The atlas stores its catalog as one Distributed Mobility Feed Registry
//! (DMFR) file per domain under `feeds/`, e.g. `feeds/bart.gov.dmfr.json`.
//! Each holds an array of feed records.
//!
//! We deserialize only the fields we actually use and ignore the rest, so an
//! upstream schema addition doesn't break the build.

use crate::onestop::OnestopId;
use crate::Result;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DmfrFile {
    #[serde(default)]
    feeds: Vec<Feed>,
    /// Operators declared at the top level rather than nested in a feed. These
    /// must carry `associated_feeds` to be of any use, and they routinely
    /// associate feeds declared in a different file.
    #[serde(default)]
    operators: Vec<Operator>,
}

/// Everything we read out of an atlas clone.
pub struct Catalog {
    pub feeds: Vec<Feed>,
    /// Top-level operators only. Operators nested in a feed stay on the feed,
    /// where their implicit association with it is still visible.
    pub operators: Vec<Operator>,
}

/// One feed record from a DMFR file.
#[derive(Debug, Clone, Deserialize)]
pub struct Feed {
    pub id: String,

    /// Which specification this feed publishes: `gtfs`, `gtfs-rt`, `gbfs`, ...
    #[serde(default = "default_spec")]
    pub spec: String,

    #[serde(default)]
    pub urls: Urls,

    #[serde(default)]
    pub operators: Vec<Operator>,

    /// Free-form key/value metadata. There is no country or location field
    /// here; the keys that hint at one are all artifacts of bulk imports from
    /// national aggregators, and too sparse to locate a feed by. Where a feed
    /// serves is measured instead - see [`crate::measure`].
    #[serde(default)]
    pub tags: BTreeMap<String, String>,

    #[serde(default)]
    pub authorization: Option<Authorization>,

    /// The DMFR file this record came from, e.g. `511.org`. Not part of the
    /// DMFR schema - we record it while loading, because the domain's TLD is
    /// sometimes the only country hint a feed has.
    #[serde(skip)]
    pub source_domain: String,
}

fn default_spec() -> String {
    "gtfs".to_owned()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Urls {
    /// Where to fetch the feed today. This is the agency's own endpoint.
    #[serde(default)]
    pub static_current: Option<String>,

    /// Previous `static_current` values, newest last. Not used for fetching -
    /// they're recorded for provenance and are frequently dead.
    #[serde(default)]
    pub static_historic: Vec<String>,

    /// GTFS-RT endpoints, present on `gtfs-rt` feeds.
    #[serde(default)]
    pub realtime_alerts: Option<String>,
    #[serde(default)]
    pub realtime_trip_updates: Option<String>,
    #[serde(default)]
    pub realtime_vehicle_positions: Option<String>,
}

impl Urls {
    /// Looks up a realtime URL by its DMFR field name, so callers can iterate
    /// over the stream kinds rather than repeat themselves per field.
    pub fn realtime_url(&self, field: &str) -> Option<&str> {
        let url = match field {
            "realtime_alerts" => &self.realtime_alerts,
            "realtime_trip_updates" => &self.realtime_trip_updates,
            "realtime_vehicle_positions" => &self.realtime_vehicle_positions,
            _ => return None,
        };
        url.as_deref().map(str::trim).filter(|u| !u.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Operator {
    pub onestop_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    /// Feeds this operator draws data from. The only link DMFR records between
    /// a GTFS-RT feed and the static feed it updates - see [`crate::realtime`].
    #[serde(default)]
    pub associated_feeds: Vec<AssociatedFeed>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

/// An entry in an operator's `associated_feeds`.
///
/// `gtfs_agency_id` also appears here, narrowing the association to one agency
/// within a feed. We don't use it: OTP keys realtime by feed, not by agency.
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
    pub fn is_gtfs(&self) -> bool {
        self.spec == "gtfs"
    }

    pub fn is_gtfs_rt(&self) -> bool {
        self.spec == "gtfs-rt"
    }

    pub fn onestop_id(&self) -> Option<OnestopId> {
        OnestopId::parse(&self.id)
    }

    /// The best geohash available for this feed: its own, else the first one
    /// from an associated operator.
    ///
    /// Operators are worth consulting because a feed may be recorded with a
    /// two-part ID while the operator nested inside it kept a v1-era
    /// three-part one. It's not many feeds, but they're free to pick up.
    pub fn geohash(&self) -> Option<crate::geohash::Geohash> {
        if let Some(geohash) = self.onestop_id().and_then(|id| id.geohash().cloned()) {
            return Some(geohash);
        }
        self.operators
            .iter()
            .find_map(|op| OnestopId::parse(&op.onestop_id).and_then(|id| id.geohash().cloned()))
    }

    /// A name to show a human curating the output.
    ///
    /// A feed can be shared by several operators - all of them are named, since
    /// any one of them alone would misrepresent what the feed covers.
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

    /// The TLD of the DMFR file this feed came from, lowercased.
    pub fn source_tld(&self) -> Option<&str> {
        self.source_domain.rsplit_once('.').map(|(_, tld)| tld)
    }
}

/// Loads every feed and top-level operator from an atlas clone.
///
/// `atlas_dir` is the root of the clone; records are read from `<root>/feeds`.
/// Files that fail to parse are an error rather than a skip - a silently
/// dropped DMFR file would look exactly like an agency that isn't cataloged.
pub fn load_catalog(atlas_dir: &Path) -> Result<Catalog> {
    let feeds_dir = atlas_dir.join("feeds");
    if !feeds_dir.is_dir() {
        return Err(format!(
            "{} doesn't look like a transitland-atlas clone: no feeds/ directory",
            atlas_dir.display()
        )
        .into());
    }

    // Sorted so the output CSV is stable across runs regardless of how the
    // filesystem happens to order directory entries.
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&feeds_dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::result::Result<_, _>>()?;
    paths.sort();

    let mut feeds = Vec::new();
    let mut operators = Vec::new();
    for path in paths {
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        let parsed: DmfrFile = serde_json::from_str(&contents)
            .map_err(|e| format!("parsing {}: {e}", path.display()))?;

        // "511.org.dmfr.json" -> "511.org"
        let domain = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .trim_end_matches(".dmfr.json")
            .to_owned();

        feeds.extend(parsed.feeds.into_iter().map(|mut feed| {
            feed.source_domain.clone_from(&domain);
            feed
        }));
        operators.extend(parsed.operators);
    }

    Ok(Catalog { feeds, operators })
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
          "urls": { "realtime_alerts": "https://example.com/alerts" }
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

    fn sample_feeds() -> Vec<Feed> {
        let parsed: DmfrFile = serde_json::from_str(SAMPLE).unwrap();
        parsed
            .feeds
            .into_iter()
            .map(|mut f| {
                f.source_domain = "511.org".to_owned();
                f
            })
            .collect()
    }

    #[test]
    fn parses_feeds() {
        let feeds = sample_feeds();
        assert_eq!(feeds.len(), 4);
        assert_eq!(feeds[0].id, "f-9q8y-sfmta");
        assert_eq!(
            feeds[0].urls.static_current.as_deref(),
            Some("http://api.511.org/transit/datafeeds?operator_id=SF")
        );
        assert_eq!(feeds[0].urls.static_historic.len(), 1);
    }

    #[test]
    fn distinguishes_specs() {
        let feeds = sample_feeds();
        assert!(feeds[0].is_gtfs());
        assert!(!feeds[0].is_gtfs_rt());
        assert!(feeds[2].is_gtfs_rt());
        assert!(!feeds[2].is_gtfs());
    }

    #[test]
    fn geohash_comes_from_the_feed_id_when_present() {
        let feeds = sample_feeds();
        assert_eq!(feeds[0].geohash().unwrap().as_str(), "9q8y");
    }

    #[test]
    fn geohash_falls_back_to_an_operator() {
        // f-smart~ca~us has no geohash of its own, but o-9qc-smart does.
        let feeds = sample_feeds();
        assert_eq!(feeds[1].geohash().unwrap().as_str(), "9qc");
    }

    #[test]
    fn parses_authorization() {
        let feeds = sample_feeds();
        let auth = feeds[0].authorization.as_ref().unwrap();
        assert_eq!(auth.kind, "query_param");
        assert_eq!(auth.param_name.as_deref(), Some("api_key"));
        assert!(feeds[1].authorization.is_none());
    }

    #[test]
    fn display_name_prefers_the_operator() {
        let feeds = sample_feeds();
        assert_eq!(feeds[1].display_name(), "Sonoma-Marin Area Rail Transit");
        // No operator to borrow a name from, so fall back to the id.
        assert_eq!(feeds[0].display_name(), "f-9q8y-sfmta");
    }

    #[test]
    fn display_name_lists_every_operator() {
        let feeds = sample_feeds();
        // The second operator has only a short_name to offer.
        assert_eq!(feeds[3].display_name(), "AC Transit, Wheels");
    }

    #[test]
    fn source_tld() {
        let feeds = sample_feeds();
        assert_eq!(feeds[0].source_tld(), Some("org"));
    }

    #[test]
    fn missing_spec_defaults_to_gtfs() {
        // Older records predate the spec field.
        let parsed: DmfrFile =
            serde_json::from_str(r#"{"feeds":[{"id":"f-x-y","urls":{}}]}"#).unwrap();
        assert!(parsed.feeds[0].is_gtfs());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let parsed: DmfrFile = serde_json::from_str(
            r#"{"feeds":[{"id":"f-x-y","spec":"gtfs","urls":{},"license":{"url":"x"},"future_field":42}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.feeds.len(), 1);
    }
}
