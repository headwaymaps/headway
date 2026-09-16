//! Credentials for feeds that won't serve their GTFS without one.
//!
//! The file is `gtfs-secrets.json`, written in transitland's `secrets.json`
//! format: a flat array of secrets, each naming the `feed_id` it unlocks. It is
//! the format transitland-lib itself reads, so the same file works here and
//! there, and it is keyed by the feed id rather than by a mangled environment
//! variable name. The name is ours; `secrets.json` alone says nothing about
//! which secrets.
//!
//! It is also how credentials are delivered:
//! `transit-credentials` writes each zone the subset it needs, and the OTP init
//! container resolves that file into `router-config.json`. Nothing in the chain
//! renames a feed along the way.

use crate::atlas::dmfr::{AuthKind, FeedCore, FeedId};
use crate::Result;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One entry of the credentials file, mirroring transitland-lib's `dmfr.Secret`.
///
/// Only the fields we can act on are modelled; unknown ones are ignored so a
/// file carrying `aws_*` or anything else transitland grows still loads.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Secret {
    pub feed_id: FeedId,
    #[serde(flatten)]
    pub value: SecretValue,
}

/// What a secret actually carries. An entry that is neither shape - a username
/// with no password, say - is refused rather than half-read.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SecretValue {
    Key { key: String },
    BasicAuth { username: String, password: String },
}

/// A blank entry for a person to fill in - deliberately not a [`Secret`],
/// which always holds something usable.
#[derive(Debug, Clone, Serialize)]
pub struct SecretTemplate {
    feed_id: FeedId,
    #[serde(flatten)]
    value: SecretValue,
}

impl SecretTemplate {
    /// Shaped for what `kind` will ask for.
    pub fn for_kind(feed_id: &FeedId, kind: &AuthKind) -> Self {
        let value = match kind {
            AuthKind::BasicAuth => SecretValue::BasicAuth {
                username: String::new(),
                password: String::new(),
            },
            AuthKind::QueryParam { .. } | AuthKind::Header { .. } => {
                SecretValue::Key { key: String::new() }
            }
        };
        Self {
            feed_id: feed_id.clone(),
            value,
        }
    }
}

impl SecretValue {
    /// Trimmed, unless it holds nothing usable.
    fn filled(self) -> Option<Self> {
        let filled = |s: String| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        match self {
            Self::Key { key } => Some(Self::Key { key: filled(key)? }),
            Self::BasicAuth { username, password } => Some(Self::BasicAuth {
                username: filled(username)?,
                password: filled(password)?,
            }),
        }
    }
}

impl Secret {}

/// Credentials, keyed by the feed each one unlocks.
#[derive(Debug, Default)]
pub struct FeedConfig {
    secrets: BTreeMap<FeedId, Secret>,
}

/// Where credentials live unless a tool is pointed somewhere else.
pub const DEFAULT_PATH: &str = "gtfs-secrets.json";

impl FeedConfig {
    /// Credentials from `gtfs-secrets.json`. A file that isn't there means we hold
    /// no credentials - which every caller already reports in its own terms -
    /// but one we can't read or parse is an error.
    pub fn from_file(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Self::parse(&contents)
                .map_err(|e| format!("reading credentials from {}: {e}", path.display()).into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => {
                Err(format!("reading credentials from {}: {error}", path.display()).into())
            }
        }
    }

    /// Parses the array of secrets. An empty file is an empty set rather than a
    /// parse error, since that is what an untouched template looks like.
    pub fn parse(contents: &str) -> Result<Self> {
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }
        let secrets: Vec<Secret> = serde_json::from_str(contents)
            .map_err(|e| format!("expected transitland's secrets.json format (an array): {e}"))?;

        let mut by_feed: BTreeMap<FeedId, Secret> = BTreeMap::new();
        for secret in secrets {
            if secret.feed_id.as_str().trim().is_empty() {
                return Err("a secret has no feed_id, so nothing can use it".into());
            }
            let Some(value) = secret.value.filled() else {
                log::debug!("{} is still blank, waiting to be filled in", secret.feed_id);
                continue;
            };
            let secret = Secret { value, ..secret };
            if let Some(previous) = by_feed.insert(secret.feed_id.clone(), secret) {
                // transitland calls this "ambiguous secrets" and refuses the feed.
                return Err(format!(
                    "two secrets both claim feed_id {:?}; transitland would call that ambiguous",
                    previous.feed_id
                )
                .into());
            }
        }
        Ok(Self { secrets: by_feed })
    }

    /// The API token for a feed, if we have a non-empty one.
    pub fn get(&self, feed_id: &FeedId) -> Option<String> {
        match &self.secret(feed_id)?.value {
            SecretValue::Key { key } => Some(key.clone()),
            SecretValue::BasicAuth { .. } => None,
        }
    }

    /// Everything we hold for a feed.
    pub fn secret(&self, feed_id: &FeedId) -> Option<&Secret> {
        self.secrets.get(feed_id)
    }

    /// Whether we hold anything usable for a feed - a key, a basic-auth pair,
    /// or a replacement url. Not every feed authenticates with a token.
    pub fn has_credential(&self, feed_id: &FeedId) -> bool {
        self.secret(feed_id).is_some()
    }

    /// The secrets for `feed_ids`, in the same format, for handing a zone just
    /// the credentials it needs. Feeds with no secret are left out.
    pub fn subset<'a>(&self, feed_ids: impl IntoIterator<Item = &'a FeedId>) -> Vec<Secret> {
        feed_ids
            .into_iter()
            .filter_map(|feed_id| self.secrets.get(feed_id))
            .cloned()
            .collect()
    }

    /// Whether anything is actually filled in.
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

impl FromIterator<(FeedId, String)> for FeedConfig {
    /// From `(feed_id, key)` pairs.
    fn from_iter<I: IntoIterator<Item = (FeedId, String)>>(iter: I) -> Self {
        Self {
            secrets: iter
                .into_iter()
                .filter_map(|(feed_id, key)| {
                    let value = SecretValue::Key { key }.filled()?;
                    Some((feed_id.clone(), Secret { feed_id, value }))
                })
                .collect(),
        }
    }
}

/// Builds a `gtfs-secrets.json` skeleton for feeds that still need a credential.
///
/// JSON carries no comments, so what each feed wants and where to ask for it is
/// reported by the caller rather than embedded here - the file has to stay
/// loadable by transitland-lib as well as by us.
pub fn template(needing_credentials: &[&FeedCore]) -> Result<String> {
    let skeleton: Vec<SecretTemplate> = needing_credentials
        .iter()
        .filter_map(|feed| {
            Some(SecretTemplate::for_kind(
                &feed.id,
                &feed.authorization.as_ref()?.kind,
            ))
        })
        .collect();

    Ok(serde_json::to_string_pretty(&skeleton)? + "\n")
}

/// What to tell someone about the feeds a template just asked for.
pub fn template_guidance(needing_credentials: &[&FeedCore]) -> String {
    let mut out = String::new();
    for feed in needing_credentials {
        let name = feed.display_name();
        out.push_str(&format!("  {}", feed.id));
        if name != feed.id.as_str() {
            out.push_str(&format!(" ({name})"));
        }
        if let Some(auth) = &feed.authorization {
            match auth.kind.param_name() {
                Some(param) => out.push_str(&format!(" - sent as {} {param:?}", auth.kind)),
                None => out.push_str(&format!(" - sent as {}", auth.kind)),
            }
            if let Some(info_url) = &auth.info_url {
                out.push_str(&format!(", request one at {info_url}"));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::atlas::dmfr::{AuthKind, Authorization, FeedCore};

    fn feed(id: &str) -> FeedCore {
        FeedCore {
            id: id.into(),
            name: None,
            operators: vec![],
            authorization: None,
        }
    }

    fn authenticated(id: &str, kind: AuthKind, info: Option<&str>) -> FeedCore {
        let mut f = feed(id);
        f.authorization = Some(Authorization {
            kind,
            info_url: info.map(str::to_owned),
        });
        f
    }

    #[test]
    fn a_feed_is_found_by_its_id_not_a_mangled_name() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-9q8y-sfmta", "key": "secret123"},
                {"feed_id": "f-sf~bay~area~rg", "key": "other"}]"#,
        )
        .unwrap();
        assert_eq!(
            config.get(&"f-9q8y-sfmta".into()),
            Some("secret123".to_owned())
        );
        assert_eq!(
            config.get(&"f-sf~bay~area~rg".into()),
            Some("other".to_owned())
        );
        assert_eq!(config.get(&"f-nonexistent".into()), None);
    }

    #[test]
    fn a_zone_is_handed_only_the_secrets_it_names() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-sf~bay~area~rg", "key": "tok"},
                {"feed_id": "f-elsewhere", "key": "other"}]"#,
        )
        .unwrap();

        let wanted: Vec<FeedId> = ["f-sf~bay~area~rg", "f-never~measured"]
            .into_iter()
            .map(FeedId::from)
            .collect();
        let subset = config.subset(&wanted);
        assert_eq!(
            subset
                .iter()
                .map(|s| s.feed_id.as_str())
                .collect::<Vec<_>>(),
            ["f-sf~bay~area~rg"]
        );
        assert_eq!(
            subset[0].value,
            SecretValue::Key {
                key: "tok".to_owned()
            }
        );
    }

    #[test]
    fn an_empty_value_is_not_a_credential() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-a", "key": ""}, {"feed_id": "f-b", "key": "  "}]"#,
        )
        .unwrap();
        assert_eq!(config.get(&"f-a".into()), None);
        assert_eq!(config.get(&"f-b".into()), None);
        assert!(config.is_empty());
    }

    #[test]
    fn a_missing_file_is_no_credentials_rather_than_an_error() {
        let config = FeedConfig::from_file(Path::new("/nonexistent/gtfs-secrets.json")).unwrap();
        assert!(config.is_empty());
        assert_eq!(config.get(&"f-anything".into()), None);
    }

    #[test]
    fn richer_secret_fields_are_carried_through() {
        let config =
            FeedConfig::parse(r#"[{"feed_id": "f-basic", "username": "u", "password": "p"}]"#)
                .unwrap();
        assert_eq!(
            config.secret(&"f-basic".into()).unwrap().value,
            SecretValue::BasicAuth {
                username: "u".to_owned(),
                password: "p".to_owned()
            }
        );
        assert!(!config.is_empty());
    }

    #[test]
    fn half_a_basic_auth_pair_is_refused() {
        let err = FeedConfig::parse(r#"[{"feed_id": "f-half", "username": "u"}]"#).unwrap_err();
        assert!(err.to_string().contains("secrets.json"), "{err}");
    }

    #[test]
    fn unknown_fields_do_not_stop_the_file_loading() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-a", "key": "k", "aws_region": "us-east-1", "filename": "x.json"}]"#,
        )
        .unwrap();
        assert_eq!(config.get(&"f-a".into()), Some("k".to_owned()));
    }

    #[test]
    fn two_secrets_for_one_feed_are_refused_the_way_transitland_would() {
        let error = FeedConfig::parse(
            r#"[{"feed_id": "f-a", "key": "one"}, {"feed_id": "f-a", "key": "two"}]"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ambiguous"), "{error}");
    }

    #[test]
    fn a_secret_without_a_feed_id_is_refused() {
        let error = FeedConfig::parse(r#"[{"key": "orphan"}]"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("feed_id"), "{error}");
    }

    #[test]
    fn the_env_format_is_no_longer_accepted_and_says_so() {
        let error = FeedConfig::parse("HEADWAY_GTFS_API_KEY_F_A=token\n")
            .unwrap_err()
            .to_string();
        assert!(error.contains("secrets.json"), "{error}");
    }

    #[test]
    fn an_empty_file_is_empty_rather_than_a_parse_error() {
        assert!(FeedConfig::parse("").unwrap().is_empty());
        assert!(FeedConfig::parse("  \n").unwrap().is_empty());
    }

    #[test]
    fn a_generated_template_round_trips() {
        let feeds = [
            authenticated(
                "f-9q8y-sfmta",
                AuthKind::QueryParam {
                    param_name: "api_key".to_owned(),
                },
                None,
            ),
            authenticated(
                "f-sf~bay~area~rg",
                AuthKind::Header {
                    param_name: "Authorization".to_owned(),
                },
                Some("https://511.org/open-data/token"),
            ),
        ];
        let refs: Vec<&FeedCore> = feeds.iter().collect();
        let text = template(&refs).unwrap();

        // A template parses, but holds no credentials - only the shape of them.
        let parsed = FeedConfig::parse(&text).unwrap();
        assert!(parsed.is_empty());
        assert!(!parsed.has_credential(&"f-9q8y-sfmta".into()));
        assert!(!parsed.has_credential(&"f-sf~bay~area~rg".into()));

        let guidance = template_guidance(&refs);
        assert!(guidance.contains("query_param"), "{guidance}");
        assert!(
            guidance.contains("https://511.org/open-data/token"),
            "{guidance}"
        );
    }

    #[test]
    fn a_basic_auth_feed_is_asked_for_a_username_and_password() {
        let feed = authenticated("f-basic", AuthKind::BasicAuth, None);
        let text = template(&[&feed]).unwrap();

        assert!(text.contains("username"), "{text}");
        assert!(text.contains("password"), "{text}");
        assert!(!text.contains("\"key\""), "{text}");
    }

    #[test]
    fn a_template_with_nothing_to_ask_for_is_an_empty_array() {
        let text = template(&[]).unwrap();
        assert!(FeedConfig::parse(&text).unwrap().is_empty());
        assert!(text.contains("[]"), "{text}");
        assert!(template_guidance(&[]).is_empty());
    }
}
