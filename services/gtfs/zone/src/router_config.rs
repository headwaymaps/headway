use crate::api_keys::api_key_env_var;
use crate::feed_id::feed_id_for;
use crate::zone::{Zone, ZoneAuth, ZoneRealtime};

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
}

impl RouterConfig {
    pub fn for_zone(zone: &Zone) -> (Self, Vec<SkippedRealtime>) {
        let mut updaters = Vec::new();
        let mut skipped = Vec::new();
        for feed in &zone.feeds {
            for realtime in &feed.realtime {
                let credential = match credential(realtime) {
                    Ok(credential) => credential,
                    Err(reason) => {
                        skipped.push(SkippedRealtime {
                            feed_id: realtime.feed_onestop_id.clone(),
                            reason,
                        });
                        continue;
                    }
                };
                for (url, kind, frequency) in streams(realtime) {
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

enum Credential {
    None,
    QueryParam { name: String, variable: String },
    Header { name: String, variable: String },
}

impl Credential {
    fn apply(&self, url: &str) -> String {
        match self {
            Self::QueryParam { name, variable } => format!(
                "{url}{}{}=${{{variable}}}",
                if url.contains('?') { '&' } else { '?' },
                name
            ),
            _ => url.to_owned(),
        }
    }

    fn headers(&self) -> Option<BTreeMap<String, String>> {
        match self {
            Self::Header { name, variable } => {
                Some(BTreeMap::from([(name.clone(), format!("${{{variable}}}"))]))
            }
            _ => None,
        }
    }
}

fn credential(realtime: &ZoneRealtime) -> Result<Credential, String> {
    let Some(auth) = realtime.authorization.as_ref() else {
        return Ok(Credential::None);
    };
    auth_credential(realtime, auth)
}

fn auth_credential(realtime: &ZoneRealtime, auth: &ZoneAuth) -> Result<Credential, String> {
    let variable = api_key_env_var(&realtime.feed_onestop_id);
    let name = auth
        .param_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned);

    match auth.kind.as_str() {
        // The catalogued URL is a placeholder the provider replaces when it
        // issues a credential, and the atlas records no parameter name for it.
        // The downloader resolves that the same way - a bare token goes on as
        // `?api_key=`, because that's what 511 (the one feed in the atlas using
        // this) actually documents. See `gtfout::measure`.
        //
        // Rendering nothing here instead would drop realtime for that feed
        // silently, which is how the Bay Area's updaters went missing.
        "replace_url" => Ok(Credential::QueryParam {
            name: name.unwrap_or_else(|| "api_key".to_owned()),
            variable,
        }),
        "query_param" => Ok(Credential::QueryParam {
            name: name.ok_or_else(missing_param_name)?,
            variable,
        }),
        "header" => Ok(Credential::Header {
            name: name.ok_or_else(missing_param_name)?,
            variable,
        }),
        kind => Err(format!("unsupported authentication type {kind:?}")),
    }
}

fn missing_param_name() -> String {
    "authentication is missing its parameter name".to_owned()
}

pub fn required_env_vars(updaters: &[Updater]) -> BTreeSet<String> {
    updaters
        .iter()
        .flat_map(|updater| {
            std::iter::once(&updater.url)
                .chain(updater.headers.iter().flat_map(|headers| headers.values()))
        })
        .filter_map(|value| value.split("${").nth(1)?.split('}').next())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::{Bounds, RealtimeUrls, Zone, ZoneFeed};

    fn zone(realtime: Vec<ZoneRealtime>) -> Zone {
        Zone {
            version: crate::zone::VERSION,
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
            credential: String::new(),
        }
    }

    /// One updater per stream the feed publishes, all pointed at the static feed
    /// they update - that id is how OTP joins them to the graph.
    #[test]
    fn each_stream_becomes_its_own_updater() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb".to_owned()),
            alerts: Some("https://example.com/a.pb".to_owned()),
            vehicle_positions: None,
        };
        let (config, skipped) = zone(vec![realtime(urls, None)]).router_config();

        assert!(skipped.is_empty());
        let kinds: Vec<&str> = config.updaters.iter().map(|u| u.kind.as_str()).collect();
        assert_eq!(kinds, ["real-time-alerts", "stop-time-updater"]);
        assert!(config
            .updaters
            .iter()
            .all(|u| u.feed_id == "headway-f-c23-kcm"));
    }

    /// The credential is a reference, not a value: OTP substitutes it from its
    /// own environment, because this ends up in a ConfigMap.
    #[test]
    fn a_query_param_credential_is_appended_as_a_placeholder() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://example.com/tu.pb?agency=1".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("query_param", Some("key")));
        let (config, _) = zone(vec![realtime(urls, auth)]).router_config();

        // `&` rather than `?`, since the URL already had a query string.
        assert_eq!(
            config.updaters[0].url,
            "https://example.com/tu.pb?agency=1&key=${HEADWAY_GTFS_API_KEY_F_C23_KCM_RT}"
        );
        assert_eq!(
            required_env_vars(&config.updaters),
            BTreeSet::from(["HEADWAY_GTFS_API_KEY_F_C23_KCM_RT".to_owned()])
        );
    }

    #[test]
    fn a_header_credential_lands_in_the_headers_map() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("header", Some("Authorization")));
        let (config, _) = zone(vec![realtime(urls, auth)]).router_config();

        let headers = config.updaters[0].headers.as_ref().unwrap();
        assert_eq!(
            headers["Authorization"],
            "${HEADWAY_GTFS_API_KEY_F_C23_KCM_RT}"
        );
        assert_eq!(config.updaters[0].url, "https://example.com/a.pb");
    }

    /// OTP has nowhere to put basic auth, so the feed is dropped - but a dropped
    /// feed has to be reported, or realtime just quietly goes missing.
    #[test]
    fn a_feed_otp_cannot_authenticate_is_skipped_by_name() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("basic_auth", Some("Authorization")));
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config();

        assert!(config.updaters.is_empty());
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].feed_id, "f-c23-kcm~rt");
        assert!(skipped[0].reason.contains("basic_auth"));
    }

    /// An authorization block with no parameter name can't be turned into a
    /// request, so it's the same story as an unsupported type.
    #[test]
    fn authentication_missing_its_parameter_name_is_skipped() {
        let urls = RealtimeUrls {
            alerts: Some("https://example.com/a.pb".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("query_param", Some("   ")));
        let (_, skipped) = zone(vec![realtime(urls, auth)]).router_config();

        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].reason.contains("parameter name"));
    }

    /// The 511 regional feed - the only `replace_url` in the atlas - records no
    /// parameter name, and OTP still has to be told where to put the token.
    /// Skipping it instead is how the Bay Area lost its realtime.
    #[test]
    fn a_replace_url_credential_falls_back_to_the_documented_parameter() {
        let urls = RealtimeUrls {
            trip_updates: Some("https://api.511.org/Transit/TripUpdates?agency=RG".to_owned()),
            ..Default::default()
        };
        let auth = Some(auth("replace_url", None));
        let (config, skipped) = zone(vec![realtime(urls, auth)]).router_config();

        assert!(skipped.is_empty());
        assert_eq!(config.updaters.len(), 1);
        assert!(config.updaters[0]
            .url
            .ends_with("?agency=RG&api_key=${HEADWAY_GTFS_API_KEY_F_C23_KCM_RT}"));
    }

    /// A zone with no realtime gets no `updaters` key at all, which is what lets
    /// bin/k8s-generate tell "no realtime" from "realtime, currently empty".
    #[test]
    fn a_zone_without_realtime_renders_an_empty_config() {
        let (config, skipped) = zone(vec![]).router_config();

        assert!(skipped.is_empty());
        assert_eq!(serde_json::to_string(&config).unwrap(), "{}");
    }
}
