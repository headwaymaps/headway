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
        .ok_or_else(|| "authentication is missing its parameter name".to_owned())?
        .to_owned();
    match auth.kind.as_str() {
        "query_param" => Ok(Credential::QueryParam { name, variable }),
        "header" => Ok(Credential::Header { name, variable }),
        kind => Err(format!("unsupported authentication type {kind:?}")),
    }
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
