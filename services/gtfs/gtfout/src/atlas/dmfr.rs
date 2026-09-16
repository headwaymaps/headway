//! Reading feed records out of a transitland-atlas clone.

use crate::Result;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    /// Why a feed in the atlas isn't one we can build from.
    pub fn unsupported_reason(&self, id: &FeedId) -> Option<&str> {
        self.unsupported
            .iter()
            .find(|feed| &feed.id == id)
            .map(|feed| feed.reason.as_str())
    }
}

/// A feed's ID: the Onestop ID the Transitland Atlas gives it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeedId(String);

/// Prints the bare id rather than `FeedId("...")`: these land in messages
/// telling a person which feed to go fix, and in JSON skeletons they paste.
impl std::fmt::Debug for FeedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl FeedId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for FeedId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl std::str::FromStr for FeedId {
    type Err = std::convert::Infallible;

    fn from_str(id: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::from(id))
    }
}

impl From<&str> for FeedId {
    fn from(id: &str) -> Self {
        Self(id.to_owned())
    }
}

/// Lets a `BTreeMap<FeedId, _>` still be looked up by `&str`.
impl std::borrow::Borrow<str> for FeedId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FeedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for FeedId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for FeedId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// What every feed record has, whatever it publishes.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedCore {
    /// The Onestop ID.
    pub id: FeedId,

    /// The operators listed inline on this record.
    #[serde(default)]
    pub operators: Vec<Operator>,

    /// The shape of the auth its URLs need - never the secret itself.
    #[serde(default)]
    pub authorization: Option<Authorization>,
}

/// A `gtfs` feed as the atlas lists it: a schedule to fetch, plus whatever
/// realtime rides along with it.
#[derive(Debug, Clone)]
pub struct StaticFeed {
    pub feed: FeedCore,

    /// Where to fetch the feed today. This is the agency's own endpoint.
    pub url: String,

    pub realtime: Vec<RealtimeFeed>,
}

impl StaticFeed {
    pub fn id(&self) -> &FeedId {
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
/// found by a spatial query - it reaches a zone only through [`StaticFeed`]
/// via [`Operator`].
#[derive(Debug, Clone)]
pub struct RealtimeFeed {
    pub feed: FeedCore,
    pub urls: RealtimeUrls,
}

/// The endpoints a realtime feed publishes.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RealtimeUrls {
    #[serde(
        default,
        alias = "realtime_trip_updates",
        skip_serializing_if = "Option::is_none"
    )]
    pub trip_updates: Option<String>,
    #[serde(
        default,
        alias = "realtime_vehicle_positions",
        skip_serializing_if = "Option::is_none"
    )]
    pub vehicle_positions: Option<String>,
    #[serde(
        default,
        alias = "realtime_alerts",
        skip_serializing_if = "Option::is_none"
    )]
    pub alerts: Option<String>,
}

/// One of the streams a GTFS-RT feed can publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StreamKind {
    #[serde(rename = "trip updates")]
    TripUpdates,
    #[serde(rename = "vehicle positions")]
    VehiclePositions,
    #[serde(rename = "alerts")]
    Alerts,
}

impl StreamKind {
    /// Every stream, in the order transit-zoner lists them.
    pub const ALL: [Self; 3] = [Self::TripUpdates, Self::VehiclePositions, Self::Alerts];
}

impl RealtimeUrls {
    /// Where to fetch one stream, if this feed publishes it.
    pub fn url(&self, kind: StreamKind) -> Option<&str> {
        match kind {
            StreamKind::TripUpdates => self.trip_updates.as_deref(),
            StreamKind::VehiclePositions => self.vehicle_positions.as_deref(),
            StreamKind::Alerts => self.alerts.as_deref(),
        }
    }

    /// Each stream this feed publishes, with where to fetch it.
    pub fn streams(&self) -> impl Iterator<Item = (StreamKind, &str)> {
        StreamKind::ALL
            .into_iter()
            .filter_map(|kind| self.url(kind).map(|url| (kind, url)))
    }

    /// Which streams this feed publishes.
    pub fn stream_kinds(&self) -> Vec<StreamKind> {
        self.streams().map(|(kind, _)| kind).collect()
    }
}

impl RealtimeFeed {
    pub fn id(&self) -> &FeedId {
        &self.feed.id
    }

    pub fn authorization(&self) -> Option<&Authorization> {
        self.feed.authorization.as_ref()
    }
}

/// A feed we don't currently handle, e.g. GBFS
#[derive(Debug, Clone)]
struct UnsupportedFeed {
    id: FeedId,
    reason: String,
}

/// What a feed record publishes, as DMFR's `spec` field says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Spec {
    Gtfs,
    GtfsRt,
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Gtfs => "gtfs",
            Self::GtfsRt => "gtfs-rt",
            Self::Other(spec) => spec,
        })
    }
}

/// One record exactly as DMFR writes it, before casting to `StaticFeed` or `RTFeed`.
#[derive(Debug, Deserialize)]
struct RawFeed {
    #[serde(flatten)]
    core: FeedCore,

    spec: Spec,

    #[serde(default)]
    urls: RawUrls,
}

#[derive(Debug, Default, Deserialize)]
struct RawUrls {
    #[serde(default)]
    static_current: Option<String>,

    #[serde(flatten)]
    realtime: RealtimeUrls,
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
    pub feed_onestop_id: Option<FeedId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthKind {
    QueryParam { param_name: String },
    Header { param_name: String },
    BasicAuth,
}

impl AuthKind {
    pub fn param_name(&self) -> Option<&str> {
        match self {
            Self::QueryParam { param_name } | Self::Header { param_name } => Some(param_name),
            Self::BasicAuth => None,
        }
    }
}

impl AuthKind {
    /// The `type` tag DMFR spells it with.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QueryParam { .. } => "query_param",
            Self::Header { .. } => "header",
            Self::BasicAuth => "basic_auth",
        }
    }
}

impl std::fmt::Display for AuthKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How to authenticate to `urls.static_current`.
///
/// Read out of DMFR, and written back out verbatim as part of the zone file -
/// see [`crate::transit_zone::zone::ZoneFeed`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    #[serde(flatten)]
    pub kind: AuthKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info_url: Option<String>,
}

impl FeedCore {
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
            self.id.to_string()
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

    let mut skip = |id: FeedId, reason: String| {
        log::debug!("skipping {id}: {reason}");
        unsupported.push(UnsupportedFeed { id, reason });
    };

    for RawFeed { core, spec, urls } in records {
        match spec {
            Spec::Gtfs => match urls.static_current.filter(|url| !url.trim().is_empty()) {
                Some(url) => gtfs_records.push((core, url)),
                None => skip(
                    core.id,
                    "it is historic or archived, with no current static url".to_owned(),
                ),
            },
            Spec::GtfsRt if urls.realtime.streams().next().is_none() => {
                skip(core.id, "it has no realtime urls to poll".to_owned())
            }
            Spec::GtfsRt => realtime.push(RealtimeFeed {
                feed: core,
                urls: urls.realtime,
            }),
            other => skip(core.id, format!("its spec is {other}")),
        }
    }

    let mut by_static = super::realtime::realtime_by_static(
        gtfs_records.iter().map(|(feed, _)| feed),
        &realtime,
        &operators,
    );

    let static_feeds = gtfs_records
        .into_iter()
        .map(|(feed, url)| StaticFeed {
            realtime: by_static.remove(&feed.id).unwrap_or_default(),
            feed,
            url,
        })
        .collect();

    unsupported.extend(realtime.into_iter().map(|rt| UnsupportedFeed {
        id: rt.feed.id,
        reason: "its spec is gtfs-rt".to_owned(),
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
    fn streams_pair_each_kind_with_its_url() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            alerts: Some("https://example.com/a.pb".to_owned()),
            vehicle_positions: None,
        };

        assert_eq!(
            urls.streams().collect::<Vec<_>>(),
            [
                (StreamKind::TripUpdates, "https://example.com/tu.pb"),
                (StreamKind::Alerts, "https://example.com/a.pb"),
            ]
        );
        assert_eq!(
            urls.stream_kinds(),
            [StreamKind::TripUpdates, StreamKind::Alerts]
        );
        assert!(RealtimeUrls::default().stream_kinds().is_empty());
    }

    #[test]
    fn parses_feeds() {
        let static_feeds = sample().static_feeds;
        assert_eq!(static_feeds.len(), 3);
        assert_eq!(static_feeds[0].id(), "f-9q8y-sfmta");
        assert_eq!(
            static_feeds[0].url,
            "http://api.511.org/transit/datafeeds?operator_id=SF"
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
        assert_eq!(
            catalog.unsupported_reason(&"f-x-bikes".into()),
            Some("its spec is gbfs")
        );
        assert_eq!(sample().unsupported_reason(&"f-9q8y-sfmta".into()), None);
        assert_eq!(sample().unsupported_reason(&"f-nonesuch".into()), None);
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
            shared.realtime[0].urls.alerts.as_deref(),
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
        assert_eq!(
            auth.kind,
            AuthKind::QueryParam {
                param_name: "api_key".to_owned()
            }
        );
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
