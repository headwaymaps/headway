//! A zone's OTP `router-config.json`, with its GTFS-RT credentials resolved.
//!
//! The credentials are read from `gtfs-secrets.json` and written into the
//! config itself, so nothing downstream has to carry them separately. That is
//! why this is rendered by the OTP init container rather than at build time:
//! the rendered config holds live tokens and must not land in a committed
//! manifest.

use crate::feed_config::FeedConfig;
use crate::measure::replace_url_token;
use crate::transit_zone::feed_id::feed_id_for;
use crate::transit_zone::zone::{Zone, ZoneAuth, ZoneRealtime};

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updaters: Vec<Updater>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Updater {
    #[serde(rename = "feedId")]
    pub feed_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub frequency: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRealtime {
    pub feed_id: String,
    pub reason: String,
    pub cause: SkipCause,
}

/// Why a realtime feed produced no updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipCause {
    /// Nothing anyone can do here: OTP cannot speak this feed's authentication.
    Unsupported,
    /// `gtfs-secrets.json` is missing, or incomplete for this feed.
    MissingCredential,
}

impl RouterConfig {
    pub fn for_zone(zone: &Zone, credentials: &FeedConfig) -> (Self, Vec<SkippedRealtime>) {
        let mut updaters = Vec::new();
        let mut skipped = Vec::new();
        for feed in &zone.feeds {
            for realtime in &feed.realtime {
                let streams: Vec<_> = streams(realtime).collect();
                if streams.is_empty() {
                    continue;
                }
                let credential = match credential(realtime, credentials, streams.len()) {
                    Ok(credential) => credential,
                    Err(skip) => {
                        skipped.push(skip);
                        continue;
                    }
                };
                for (url, kind, frequency) in streams {
                    updaters.push(Updater {
                        feed_id: feed_id_for(&feed.feed_onestop_id),
                        kind: kind.to_owned(),
                        frequency: format!("{frequency}s"),
                        url: credential.apply(url),
                        headers: credential.headers(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Everything the zone needs, static feeds and realtime alike. What
    /// `bin/transit-credentials` copies into the zone's `gtfs-secrets.json`.
    All,
    /// Only what OpenTripPlanner will look for. What decides whether a
    /// deployment's Secret is `optional`.
    Runtime,
}

/// The feed ids whose secrets this zone needs, as `gtfs-secrets.json` keys them.
pub fn required_feeds(zone: &Zone, scope: Scope) -> BTreeSet<String> {
    let mut feeds = credentialed_realtime_feeds(zone);

    if scope == Scope::All {
        for feed in &zone.feeds {
            if feed.authorization.is_some() {
                feeds.insert(feed.feed_onestop_id.clone());
            }
        }
    }

    feeds
}

/// The realtime feeds whose credential OTP will actually use: the ones that
/// publish an endpoint and authenticate in a way OTP can speak.
fn credentialed_realtime_feeds(zone: &Zone) -> BTreeSet<String> {
    zone.feeds
        .iter()
        .flat_map(|feed| &feed.realtime)
        .filter(|realtime| streams(realtime).next().is_some())
        .filter(|realtime| {
            realtime
                .authorization
                .as_ref()
                .is_some_and(|auth| is_supported(&auth.kind))
        })
        .map(|realtime| realtime.feed_onestop_id.clone())
        .collect()
}

fn is_supported(kind: &str) -> bool {
    matches!(
        kind,
        "query_param" | "header" | "basic_auth" | "replace_url"
    )
}

fn streams(realtime: &ZoneRealtime) -> impl Iterator<Item = (&str, &'static str, u32)> {
    [
        (realtime.urls.alerts.as_deref(), "real-time-alerts", 300),
        (
            realtime.urls.trip_updates.as_deref(),
            "stop-time-updater",
            60,
        ),
        (
            realtime.urls.vehicle_positions.as_deref(),
            "vehicle-positions",
            60,
        ),
    ]
    .into_iter()
    .filter_map(|(url, kind, frequency)| url.map(|url| (url, kind, frequency)))
}

/// A resolved credential: the live values OTP will send, not a placeholder.
#[derive(Default)]
struct Credential {
    /// A url to fetch instead of the one the zone records.
    url_override: Option<String>,
    query: Option<(String, String)>,
    header: Option<(String, String)>,
}

impl Credential {
    fn apply(&self, url: &str) -> String {
        let url = self.url_override.as_deref().unwrap_or(url);
        match &self.query {
            Some((name, value)) => format!(
                "{url}{}{name}={value}",
                if url.contains('?') { '&' } else { '?' }
            ),
            None => url.to_owned(),
        }
    }

    fn headers(&self) -> Option<BTreeMap<String, String>> {
        let (name, value) = self.header.as_ref()?;
        Some(BTreeMap::from([(name.clone(), value.clone())]))
    }
}

fn credential(
    realtime: &ZoneRealtime,
    credentials: &FeedConfig,
    stream_count: usize,
) -> Result<Credential, SkippedRealtime> {
    let Some(auth) = realtime.authorization.as_ref() else {
        return Ok(Credential::default());
    };
    resolve(realtime, auth, credentials, stream_count)
}

fn resolve(
    realtime: &ZoneRealtime,
    auth: &ZoneAuth,
    credentials: &FeedConfig,
    stream_count: usize,
) -> Result<Credential, SkippedRealtime> {
    let feed_id = realtime.feed_onestop_id.as_str();
    let skip = |cause, reason: String| SkippedRealtime {
        feed_id: feed_id.to_owned(),
        reason,
        cause,
    };
    let missing = |reason: String| skip(SkipCause::MissingCredential, reason);

    if !is_supported(&auth.kind) {
        return Err(skip(
            SkipCause::Unsupported,
            format!("unsupported authentication type {:?}", auth.kind),
        ));
    }

    let Some(secret) = credentials.secret(feed_id) else {
        return Err(missing(format!(
            "no credential; add an entry for {feed_id:?} to gtfs-secrets.json"
        )));
    };

    let mut credential = Credential::default();

    // A replace_url swaps the url out whatever the declared scheme is, the way
    // transitland's MatchSecrets does, so it is honored before the rest. A
    // realtime feed publishes up to three endpoints under one id, though, and
    // one replacement cannot stand in for all of them.
    if let Some(replacement) = secret.replace_url() {
        if stream_count > 1 {
            return Err(skip(
                SkipCause::Unsupported,
                format!(
                    "its gtfs-secrets.json entry replaces the url, but the feed publishes \
                     {stream_count} endpoints and one replacement cannot serve them all"
                ),
            ));
        }
        credential.url_override = Some(replacement.to_owned());
    }

    let key = || {
        secret.key().ok_or_else(|| {
            missing(format!(
                "gtfs-secrets.json entry for {feed_id:?} has no \"key\""
            ))
        })
    };
    let param_name = |fallback: &str| {
        auth.param_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(fallback)
            .to_owned()
    };

    match auth.kind.as_str() {
        "query_param" => credential.query = Some((param_name("api_key"), key()?.to_owned())),
        "header" => credential.header = Some((param_name("Authorization"), key()?.to_owned())),
        "basic_auth" => {
            let Some((user, password)) = secret.basic_auth() else {
                return Err(missing(format!(
                    "gtfs-secrets.json entry for {feed_id:?} needs \"username\" and \"password\""
                )));
            };
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
            credential.header = Some(("Authorization".to_owned(), format!("Basic {encoded}")));
        }
        "replace_url" => {
            // The url itself is the credential. When the secret carries no
            // replacement, the key is appended as a query param instead, which
            // is how 511-style feeds have always worked here.
            if credential.url_override.is_none() {
                let token = replace_url_token(key()?)
                    .map_err(|error| missing(format!("{feed_id}: {error}")))?;
                credential.query = Some((param_name("api_key"), token.to_owned()));
            }
        }
        kind => unreachable!("{kind:?} passed is_supported but has no branch"),
    }

    Ok(credential)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transit_zone::zone::{Bounds, RealtimeUrls, Zone, ZoneFeed};

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
                feed_onestop_id: "f-c23-kcm".to_owned(),
                provider: "King County Metro".to_owned(),
                url: "https://example.com/kcm.zip".to_owned(),
                authorization: None,
                realtime,
            }],
        }
    }

    fn realtime(urls: RealtimeUrls, authorization: Option<ZoneAuth>) -> ZoneRealtime {
        ZoneRealtime {
            feed_onestop_id: "f-c23-kcm~rt".to_owned(),
            urls,
            authorization,
        }
    }

    fn auth(kind: &str, param_name: Option<&str>) -> ZoneAuth {
        ZoneAuth {
            kind: kind.to_owned(),
            param_name: param_name.map(str::to_owned),
            info_url: None,
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
        assert!(config
            .updaters
            .iter()
            .all(|u| u.feed_id == "headway-f-c23-kcm"));
    }

    #[test]
    fn a_query_param_credential_is_written_into_the_url() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb?agency=1".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("query_param", Some("key")));
        let (config, _) = zone(vec![realtime(urls, auth)]).router_config(&token());

        assert_eq!(
            config.updaters[0].url,
            "https://example.com/tu.pb?agency=1&key=s3cret"
        );
    }

    #[test]
    fn a_query_param_with_no_name_falls_back_to_the_documented_one() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("query_param", Some("   ")));
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&token());

        assert!(skipped.is_empty());
        assert_eq!(
            config.updaters[0].url,
            "https://example.com/tu.pb?api_key=s3cret"
        );
    }

    #[test]
    fn a_header_credential_lands_in_the_headers_map() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("header", Some("X-Api-Key")));
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
        let auth = Some(auth("basic_auth", None));
        let credentials =
            secrets(r#"[{"feed_id": "f-c23-kcm~rt", "username": "u", "password": "p"}]"#);
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&credentials);

        assert!(skipped.is_empty());
        let headers = config.updaters[0].headers.as_ref().unwrap();
        // base64("u:p")
        assert_eq!(headers["Authorization"], "Basic dTpw");
    }

    #[test]
    fn a_feed_otp_cannot_authenticate_is_skipped_by_name() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("oauth", Some("Authorization")));
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&token());

        assert!(config.updaters.is_empty());
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].feed_id, "f-c23-kcm~rt");
        assert_eq!(skipped[0].cause, SkipCause::Unsupported);
        assert!(skipped[0].reason.contains("oauth"));
    }

    #[test]
    fn a_missing_secret_is_distinguishable_from_one_we_cannot_use() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("query_param", Some("key")));
        let (config, skipped) =
            zone(vec![realtime(urls, auth)]).router_config(&FeedConfig::default());

        assert!(config.updaters.is_empty());
        assert_eq!(skipped[0].cause, SkipCause::MissingCredential);
        assert!(skipped[0].reason.contains("gtfs-secrets.json"));
    }

    #[test]
    fn a_replace_url_credential_falls_back_to_the_documented_parameter() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://api.511.org/Transit/TripUpdates?agency=RG".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("replace_url", None));
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&token());

        assert!(skipped.is_empty());
        assert_eq!(config.updaters.len(), 1);
        assert!(config.updaters[0]
            .url
            .ends_with("?agency=RG&api_key=s3cret"));
    }

    #[test]
    fn a_replacement_url_stands_in_for_a_feeds_only_endpoint() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/public.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("replace_url", None));
        let credentials = secrets(
            r#"[{"feed_id": "f-c23-kcm~rt", "replace_url": "https://example.com/private.pb?t=x"}]"#,
        );
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&credentials);

        assert!(skipped.is_empty());
        assert_eq!(config.updaters[0].url, "https://example.com/private.pb?t=x");
    }

    #[test]
    fn a_replacement_url_cannot_stand_in_for_several_endpoints() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("replace_url", None));
        let credentials =
            secrets(r#"[{"feed_id": "f-c23-kcm~rt", "replace_url": "https://example.com/x.pb"}]"#);
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config(&credentials);

        assert!(config.updaters.is_empty());
        assert_eq!(skipped[0].cause, SkipCause::Unsupported);
        assert!(
            skipped[0].reason.contains("2 endpoints"),
            "{}",
            skipped[0].reason
        );
    }

    #[test]
    fn the_cluster_needs_only_the_realtime_credentials() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            ..Default::default()
        };
        let mut zone = zone(vec![realtime(urls, Some(auth("replace_url", None)))]);
        zone.feeds[0].authorization = Some(auth("replace_url", None));

        assert_eq!(
            required_feeds(&zone, Scope::Runtime),
            BTreeSet::from(["f-c23-kcm~rt".to_owned()])
        );
        assert_eq!(
            required_feeds(&zone, Scope::All),
            BTreeSet::from(["f-c23-kcm".to_owned(), "f-c23-kcm~rt".to_owned()])
        );
    }

    #[test]
    fn only_feeds_otp_can_authenticate_are_counted_as_needing_a_credential() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let zone = zone(vec![
            realtime(urls.clone(), Some(auth("query_param", Some("key")))),
            ZoneRealtime {
                feed_onestop_id: "f-c23-kcm~oauth".to_owned(),
                urls: urls.clone(),
                authorization: Some(auth("oauth", None)),
            },
            ZoneRealtime {
                feed_onestop_id: "f-c23-kcm~open".to_owned(),
                urls,
                authorization: None,
            },
            ZoneRealtime {
                feed_onestop_id: "f-c23-kcm~silent".to_owned(),
                urls: RealtimeUrls::default(),
                authorization: Some(auth("query_param", Some("key"))),
            },
        ]);

        assert_eq!(
            required_feeds(&zone, Scope::All),
            BTreeSet::from(["f-c23-kcm~rt".to_owned()])
        );
    }

    #[test]
    fn a_zone_without_realtime_renders_an_empty_config() {
        let (config, skipped) = zone(vec![]).router_config(&FeedConfig::default());

        assert!(skipped.is_empty());
        assert_eq!(serde_json::to_string(&config).unwrap(), "{}");
    }
}
