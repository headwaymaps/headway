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

impl RouterConfig {
    pub fn for_zone(zone: &Zone, credentials: &FeedConfig) -> (Self, Vec<SkippedRealtime>) {
        let mut updaters = Vec::new();
        let mut skipped = Vec::new();
        for feed in &zone.feeds {
            for realtime in &feed.realtime {
                let streams: Vec<_> = realtime.urls.streams().collect();
                if streams.is_empty() {
                    continue;
                }
                let credential = match credential(realtime, credentials) {
                    Ok(credential) => credential,
                    Err(skip) => {
                        skipped.push(skip);
                        continue;
                    }
                };
                for (stream, url) in streams {
                    let (kind, frequency) = otp_updater(stream);
                    updaters.push(Updater {
                        feed_id: feed.feed_onestop_id.clone(),
                        kind: kind.to_owned(),
                        frequency: format!("{frequency}s"),
                        url: match &credential {
                            Some(credential) => credential.url(url),
                            None => url.to_owned(),
                        },
                        headers: credential.as_ref().and_then(AuthKindSecret::headers),
                    });
                }
            }
        }
        updaters.sort();
        updaters.dedup();
        (Self { updaters }, skipped)
    }
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

/// What OTP calls a stream, and how often it should poll it. Our policy, not
/// anything the atlas says, which is why it lives here rather than on
/// [`StreamKind`].
fn otp_updater(kind: StreamKind) -> (&'static str, u32) {
    match kind {
        StreamKind::TripUpdates => ("stop-time-updater", 60),
        StreamKind::VehiclePositions => ("vehicle-positions", 60),
        StreamKind::Alerts => ("real-time-alerts", 300),
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
        ZoneRealtime {
            feed_onestop_id: "f-c23-kcm~rt".into(),
            urls,
            authorization,
        }
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
    fn each_stream_becomes_its_own_updater() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            alerts: Some("https://example.com/a.pb".to_owned()),
            vehicle_positions: None,
        };
        let (config, skipped) =
            zone(vec![realtime(urls, None)]).router_config(&FeedConfig::default());

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
        let (config, _) = zone(vec![realtime(urls, auth)]).router_config(&token());

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
        let (config, _) = zone(vec![realtime(urls, auth)]).router_config(&token());

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
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&credentials);

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
        let (config, skipped) =
            zone(vec![realtime(urls, auth)]).router_config(&FeedConfig::default());

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
        let (config, skipped) = zone(vec![]).router_config(&FeedConfig::default());

        assert!(skipped.is_empty());
        assert_eq!(serde_json::to_string(&config).unwrap(), "{}");
    }
}
