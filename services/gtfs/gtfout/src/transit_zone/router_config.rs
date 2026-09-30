//! A zone's OTP `router-config.json`, with its GTFS-RT credentials resolved.
//!
//! The credentials are read from `gtfs-secrets.json` and written into the
//! config itself, so nothing downstream has to carry them separately. That is
//! why this is rendered by the OTP init container rather than at build time:
//! the rendered config holds live tokens and must not land in a committed
//! manifest.

use crate::atlas::dmfr::{FeedId, StreamKind};
use crate::auth_kind_secret::AuthKindSecret;
use crate::feed_config::FeedConfig;
use crate::transit_zone::zone::{Zone, ZoneRealtime};

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updaters: Vec<Updater>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Updater {
    #[serde(rename = "feedId")]
    pub feed_id: FeedId,
    #[serde(rename = "type")]
    pub kind: String,
    pub frequency: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
}

/// A realtime feed left out of the config because `gtfs-secrets.json` holds
/// nothing usable for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRealtime {
    pub feed_id: FeedId,
    pub reason: String,
}

/// One agency's stream, with its credential already folded into the request.
struct Source {
    url: String,
    headers: Option<BTreeMap<String, String>>,
}

impl RouterConfig {
    /// Refuses a static feed carrying more than one realtime feed for a stream. OTP keys realtime
    /// data by static feed and replaces it on every poll, so the second feed would spend its life
    /// erasing the first.
    pub fn for_zone(
        zone: &Zone,
        credentials: &FeedConfig,
    ) -> std::result::Result<(Self, Vec<SkippedRealtime>), String> {
        let mut updaters = Vec::new();
        let mut skipped = Vec::new();
        for feed in &zone.feeds {
            let mut resolved = Vec::new();
            for realtime in &feed.realtime {
                if realtime.urls.streams().next().is_none() {
                    continue;
                }
                match credential(realtime, credentials) {
                    Ok(credential) => resolved.push((realtime, credential)),
                    Err(skip) => skipped.push(skip),
                }
            }

            for stream in StreamKind::ALL {
                let mut sources = sources_for(&resolved, stream);
                if sources.len() > 1 {
                    return Err(format!(
                        "{} has {} realtime feeds publishing {}, and OTP holds one per feed - each \
                         poll would erase the last. Name the one feed that covers this schedule.",
                        feed.feed_onestop_id,
                        sources.len(),
                        stream.zone_key(),
                    ));
                }
                let Some(source) = sources.pop() else {
                    continue;
                };
                updaters.push(Updater {
                    feed_id: feed.feed_onestop_id.clone(),
                    kind: stream.otp_updater_type().to_owned(),
                    frequency: format!("{}s", stream.poll_seconds()),
                    url: source.url,
                    headers: source.headers,
                });
            }
        }
        updaters.sort();
        updaters.dedup();
        Ok((Self { updaters }, skipped))
    }
}

/// Every agency publishing one stream into a static feed, one entry per distinct url.
///
/// A url two realtime feeds share is one stream to fetch, not two - King County Metro and the
/// Seattle Streetcar publish into the same bucket.
fn sources_for(
    resolved: &[(&ZoneRealtime, Option<AuthKindSecret>)],
    stream: StreamKind,
) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();
    for (realtime, credential) in resolved {
        let Some(url) = realtime.urls.url(stream) else {
            continue;
        };
        let url = match credential {
            Some(credential) => credential.url(url),
            None => url.to_owned(),
        };
        if sources.iter().any(|existing| existing.url == url) {
            continue;
        }
        sources.push(Source {
            url,
            headers: credential.as_ref().and_then(AuthKindSecret::headers),
        });
    }
    sources
}

/// Which of a zone's credentials to name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Scope {
    /// Everything the zone needs, static feeds and realtime alike. What
    /// `bin/transit-credentials` copies into the zone's `gtfs-secrets.json`.
    All,
    /// Only what OpenTripPlanner will look for. What decides whether a
    /// deployment's Secret is `optional`.
    Runtime,
}

impl Zone {
    /// The feed ids whose secrets this zone needs, as `gtfs-secrets.json` keys
    /// them.
    pub fn required_feeds(&self, scope: Scope) -> BTreeSet<FeedId> {
        let mut feeds = self.credentialed_realtime_feeds();

        if scope == Scope::All {
            for feed in &self.feeds {
                if feed.authorization.is_some() {
                    feeds.insert(feed.feed_onestop_id.clone());
                }
            }
        }

        feeds
    }

    /// The realtime feeds whose credential OTP will actually use: the ones that
    /// publish an endpoint and authenticate in a way OTP can speak.
    fn credentialed_realtime_feeds(&self) -> BTreeSet<FeedId> {
        self.feeds
            .iter()
            .flat_map(|feed| &feed.realtime)
            .filter(|realtime| realtime.urls.streams().next().is_some())
            .filter(|realtime| realtime.authorization.is_some())
            .map(|realtime| realtime.feed_onestop_id.clone())
            .collect()
    }
}

/// What a stream is called and how often it's read. Our policy rather than anything the atlas
/// says, so it lives beside the config it shapes.
impl StreamKind {
    /// What OTP calls the updater for this stream.
    fn otp_updater_type(self) -> &'static str {
        match self {
            Self::TripUpdates => "stop-time-updater",
            Self::VehiclePositions => "vehicle-positions",
            Self::Alerts => "real-time-alerts",
        }
    }

    /// How often this stream is worth re-reading.
    fn poll_seconds(self) -> u32 {
        match self {
            Self::TripUpdates | Self::VehiclePositions => 60,
            Self::Alerts => 300,
        }
    }

    /// How the zone file spells this stream, for naming the one a feed doubled up on.
    fn zone_key(self) -> &'static str {
        match self {
            Self::TripUpdates => "trip_updates",
            Self::VehiclePositions => "vehicle_positions",
            Self::Alerts => "alerts",
        }
    }
}

fn credential(
    realtime: &ZoneRealtime,
    credentials: &FeedConfig,
) -> Result<Option<AuthKindSecret>, SkippedRealtime> {
    let Some(auth) = realtime.authorization.as_ref() else {
        return Ok(None);
    };
    let feed_id = &realtime.feed_onestop_id;
    let skip = |reason: String| SkippedRealtime {
        feed_id: feed_id.clone(),
        reason,
    };
    let Some(secret) = credentials.secret(feed_id) else {
        return Err(skip(format!(
            "no credential; add an entry for {feed_id:?} to gtfs-secrets.json"
        )));
    };

    AuthKindSecret::resolve(&auth.kind, secret, feed_id)
        .map(Some)
        .map_err(skip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::dmfr::AuthKind;
    use crate::atlas::dmfr::{Authorization, RealtimeUrls};
    use crate::transit_zone::zone::{Bounds, Zone, ZoneFeed};

    fn zone(realtime: Vec<ZoneRealtime>) -> Zone {
        Zone {
            version: crate::transit_zone::zone::VERSION,
            bounds: Bounds {
                min_lon: -122.5,
                min_lat: 47.3,
                max_lon: -122.0,
                max_lat: 47.8,
            },
            feeds: vec![ZoneFeed {
                feed_onestop_id: "f-c23-kcm".into(),
                provider: "King County Metro".to_owned(),
                url: "https://example.com/kcm.zip".to_owned(),
                authorization: None,
                realtime,
            }],
        }
    }

    fn realtime(urls: RealtimeUrls, authorization: Option<Authorization>) -> ZoneRealtime {
        named_realtime("f-c23-kcm~rt", urls, authorization)
    }

    fn named_realtime(
        feed_onestop_id: &str,
        urls: RealtimeUrls,
        authorization: Option<Authorization>,
    ) -> ZoneRealtime {
        ZoneRealtime {
            feed_onestop_id: feed_onestop_id.into(),
            urls,
            authorization,
        }
    }

    /// One agency publishing every stream at its own urls.
    fn agency(feed_onestop_id: &str, host: &str) -> ZoneRealtime {
        named_realtime(
            feed_onestop_id,
            RealtimeUrls {
                trip_updates: Some(format!("https://{host}/tu.pb")),
                vehicle_positions: Some(format!("https://{host}/vp.pb")),
                alerts: Some(format!("https://{host}/a.pb")),
            },
            None,
        )
    }

    fn updaters_of(config: &RouterConfig, kind: &str) -> Vec<String> {
        config
            .updaters
            .iter()
            .filter(|u| u.kind == kind)
            .map(|u| u.url.clone())
            .collect()
    }

    fn auth(kind: AuthKind) -> Authorization {
        Authorization {
            kind,
            info_url: None,
        }
    }

    fn query_param(param_name: &str) -> AuthKind {
        AuthKind::QueryParam {
            param_name: param_name.to_owned(),
        }
    }

    fn header(param_name: &str) -> AuthKind {
        AuthKind::Header {
            param_name: param_name.to_owned(),
        }
    }

    fn secrets(json: &str) -> FeedConfig {
        FeedConfig::parse(json).unwrap()
    }

    fn token() -> FeedConfig {
        secrets(r#"[{"feed_id": "f-c23-kcm~rt", "key": "s3cret"}]"#)
    }

    #[test]
    fn a_lone_agency_is_polled_directly() {
        let zone = zone(vec![agency("f-kcm~rt", "kcm.example")]);
        let (config, _) = zone.router_config(&FeedConfig::default()).unwrap();

        assert_eq!(
            updaters_of(&config, "vehicle-positions"),
            ["https://kcm.example/vp.pb"]
        );
    }

    #[test]
    fn two_agencies_publishing_the_same_url_are_one_stream_to_fetch() {
        let zone = zone(vec![
            agency("f-kcm~rt", "kcm.example"),
            agency("f-streetcar~rt", "kcm.example"),
        ]);
        let (config, _) = zone.router_config(&FeedConfig::default()).unwrap();

        assert_eq!(
            updaters_of(&config, "vehicle-positions"),
            ["https://kcm.example/vp.pb"]
        );
    }

    /// OTP would let the second updater erase the first on every poll, so the zone is refused
    /// rather than deployed half-working. Alerts included: nothing in a committed zone needs two
    /// of those either, and a rule with an exception is a rule nobody remembers.
    #[test]
    fn two_realtime_feeds_for_one_stream_is_refused() {
        let zone = zone(vec![
            agency("f-kcm~rt", "kcm.example"),
            agency("f-st~rt", "st.example"),
        ]);
        let error = zone
            .router_config(&FeedConfig::default())
            .expect_err("two feeds on one stream cannot both reach OTP");

        assert!(error.contains("f-c23-kcm"), "{error}");
        assert!(
            error.contains("vehicle_positions")
                || error.contains("trip_updates")
                || error.contains("alerts"),
            "{error}"
        );
    }

    #[test]
    fn each_stream_becomes_its_own_updater() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            alerts: Some("https://example.com/a.pb".to_owned()),
            vehicle_positions: None,
        };
        let (config, skipped) = zone(vec![realtime(urls, None)])
            .router_config(&FeedConfig::default())
            .unwrap();

        assert!(skipped.is_empty());
        let kinds: Vec<&str> = config.updaters.iter().map(|u| u.kind.as_str()).collect();
        assert_eq!(kinds, ["real-time-alerts", "stop-time-updater"]);
        assert!(config.updaters.iter().all(|u| u.feed_id == "f-c23-kcm"));
    }

    #[test]
    fn a_query_param_credential_is_written_into_the_url() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb?agency=1".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth(query_param("key")));
        let (config, _) = zone(vec![realtime(urls, auth)])
            .router_config(&token())
            .unwrap();

        assert_eq!(
            config.updaters[0].url,
            "https://example.com/tu.pb?agency=1&key=s3cret"
        );
    }

    #[test]
    fn a_header_credential_lands_in_the_headers_map() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth(header("X-Api-Key")));
        let (config, _) = zone(vec![realtime(urls, auth)])
            .router_config(&token())
            .unwrap();

        let headers = config.updaters[0].headers.as_ref().unwrap();
        assert_eq!(headers["X-Api-Key"], "s3cret");
        assert_eq!(config.updaters[0].url, "https://example.com/a.pb");
    }

    #[test]
    fn basic_auth_becomes_an_authorization_header() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth(AuthKind::BasicAuth));
        let credentials =
            secrets(r#"[{"feed_id": "f-c23-kcm~rt", "username": "u", "password": "p"}]"#);
        let (config, skipped) = zone(vec![realtime(urls, auth)])
            .router_config(&credentials)
            .unwrap();

        assert!(skipped.is_empty());
        let headers = config.updaters[0].headers.as_ref().unwrap();
        // base64("u:p")
        assert_eq!(headers["Authorization"], "Basic dTpw");
    }

    #[test]
    fn a_missing_secret_is_distinguishable_from_one_we_cannot_use() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth(query_param("key")));
        let (config, skipped) = zone(vec![realtime(urls, auth)])
            .router_config(&FeedConfig::default())
            .unwrap();

        assert!(config.updaters.is_empty());
        assert!(skipped[0].reason.contains("gtfs-secrets.json"));
    }

    #[test]
    fn the_cluster_needs_only_the_realtime_credentials() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            ..Default::default()
        };
        let mut zone = zone(vec![realtime(urls, Some(auth(query_param("key"))))]);
        zone.feeds[0].authorization = Some(auth(query_param("key")));

        assert_eq!(
            zone.required_feeds(Scope::Runtime),
            BTreeSet::from(["f-c23-kcm~rt".into()])
        );
        assert_eq!(
            zone.required_feeds(Scope::All),
            BTreeSet::from(["f-c23-kcm".into(), "f-c23-kcm~rt".into()])
        );
    }

    #[test]
    fn only_feeds_otp_can_authenticate_are_counted_as_needing_a_credential() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let zone = zone(vec![
            realtime(urls.clone(), Some(auth(query_param("key")))),
            ZoneRealtime {
                feed_onestop_id: "f-c23-kcm~open".into(),
                urls,
                authorization: None,
            },
            ZoneRealtime {
                feed_onestop_id: "f-c23-kcm~silent".into(),
                urls: RealtimeUrls::default(),
                authorization: Some(auth(query_param("key"))),
            },
        ]);

        assert_eq!(
            zone.required_feeds(Scope::All),
            BTreeSet::from(["f-c23-kcm~rt".into()])
        );
    }

    #[test]
    fn a_zone_without_realtime_renders_an_empty_config() {
        let (config, skipped) = zone(vec![]).router_config(&FeedConfig::default()).unwrap();

        assert!(skipped.is_empty());
        assert_eq!(serde_json::to_string(&config).unwrap(), "{}");
    }
}
