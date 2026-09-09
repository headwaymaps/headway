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

use crate::atlas::dmfr::Feed;
use crate::Result;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One entry of the credentials file, mirroring transitland-lib's `dmfr.Secret`.
///
/// Only the fields we can act on are modelled; unknown ones are ignored so a
/// file carrying `aws_*` or anything else transitland grows still loads.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Secret {
    pub feed_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// A url to fetch *instead of* the one in the atlas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_type: Option<String>,
}

fn filled(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

impl Secret {
    /// The API token, if this secret carries a non-blank one.
    pub fn key(&self) -> Option<&str> {
        filled(&self.key)
    }

    /// Username and password, if both are present.
    pub fn basic_auth(&self) -> Option<(&str, &str)> {
        Some((filled(&self.username)?, filled(&self.password)?))
    }

    /// The url to fetch instead of the atlas's, if any.
    pub fn replace_url(&self) -> Option<&str> {
        filled(&self.replace_url)
    }

    /// Whether this secret can satisfy anything at all.
    fn is_blank(&self) -> bool {
        self.key().is_none() && self.basic_auth().is_none() && self.replace_url().is_none()
    }
}

/// Credentials, keyed by the feed each one unlocks.
#[derive(Debug, Default)]
pub struct FeedConfig {
    secrets: BTreeMap<String, Secret>,
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

        let mut by_feed: BTreeMap<String, Secret> = BTreeMap::new();
        for secret in secrets {
            if secret.feed_id.trim().is_empty() {
                return Err("a secret has no feed_id, so nothing can use it".into());
            }
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
    pub fn get(&self, feed_id: &str) -> Option<String> {
        self.secret(feed_id)?.key().map(str::to_owned)
    }

    /// Everything we hold for a feed.
    pub fn secret(&self, feed_id: &str) -> Option<&Secret> {
        self.secrets.get(feed_id)
    }

    /// Whether we hold anything usable for a feed - a key, a basic-auth pair,
    /// or a replacement url. Not every feed authenticates with a token.
    pub fn has_credential(&self, feed_id: &str) -> bool {
        self.secret(feed_id)
            .is_some_and(|secret| !secret.is_blank())
    }

    /// The secrets for `feed_ids`, in the same format, for handing a zone just
    /// the credentials it needs. Feeds with no secret are left out.
    pub fn subset<'a>(&self, feed_ids: impl IntoIterator<Item = &'a str>) -> Vec<Secret> {
        feed_ids
            .into_iter()
            .filter_map(|feed_id| self.secrets.get(feed_id))
            .cloned()
            .collect()
    }

    /// Whether anything is actually filled in.
    pub fn is_empty(&self) -> bool {
        self.secrets.values().all(Secret::is_blank)
    }
}

impl FromIterator<(String, String)> for FeedConfig {
    /// From `(feed_id, key)` pairs.
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self {
            secrets: iter
                .into_iter()
                .map(|(feed_id, key)| {
                    (
                        feed_id.clone(),
                        Secret {
                            feed_id,
                            key: Some(key),
                            ..Secret::default()
                        },
                    )
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
pub fn template(needing_credentials: &[&Feed]) -> Result<String> {
    let skeleton: Vec<Secret> = needing_credentials
        .iter()
        .map(|feed| Secret {
            feed_id: feed.id.clone(),
            key: Some(String::new()),
            ..Secret::default()
        })
        .collect();

    Ok(serde_json::to_string_pretty(&skeleton)? + "\n")
}

/// What to tell someone about the feeds a template just asked for.
pub fn template_guidance(needing_credentials: &[&Feed]) -> String {
    let mut out = String::new();
    for feed in needing_credentials {
        let name = feed.display_name();
        out.push_str(&format!("  {}", feed.id));
        if name != feed.id {
            out.push_str(&format!(" ({name})"));
        }
        if let Some(auth) = &feed.authorization {
            match &auth.param_name {
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

    use crate::atlas::dmfr::{Authorization, Feed};

    fn feed(id: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            operators: vec![],
            authorization: None,
        }
    }

    fn authenticated(id: &str, kind: &str, param: Option<&str>, info: Option<&str>) -> Feed {
        let mut f = feed(id);
        f.authorization = Some(Authorization {
            kind: kind.to_owned(),
            param_name: param.map(str::to_owned),
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
        assert_eq!(config.get("f-9q8y-sfmta"), Some("secret123".to_owned()));
        assert_eq!(config.get("f-sf~bay~area~rg"), Some("other".to_owned()));
        assert_eq!(config.get("f-nonexistent"), None);
    }

    #[test]
    fn a_zone_is_handed_only_the_secrets_it_names() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-sf~bay~area~rg", "key": "tok"},
                {"feed_id": "f-elsewhere", "key": "other"}]"#,
        )
        .unwrap();

        let subset = config.subset(["f-sf~bay~area~rg", "f-never~measured"]);
        assert_eq!(
            subset
                .iter()
                .map(|s| s.feed_id.as_str())
                .collect::<Vec<_>>(),
            ["f-sf~bay~area~rg"]
        );
        assert_eq!(subset[0].key(), Some("tok"));
    }

    #[test]
    fn an_empty_value_is_not_a_credential() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-a", "key": ""}, {"feed_id": "f-b", "key": "  "}]"#,
        )
        .unwrap();
        assert_eq!(config.get("f-a"), None);
        assert_eq!(config.get("f-b"), None);
        assert!(config.is_empty());
    }

    #[test]
    fn a_missing_file_is_no_credentials_rather_than_an_error() {
        let config = FeedConfig::from_file(Path::new("/nonexistent/gtfs-secrets.json")).unwrap();
        assert!(config.is_empty());
        assert_eq!(config.get("f-anything"), None);
    }

    #[test]
    fn richer_secret_fields_are_carried_through() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-basic", "username": "u", "password": "p"},
                {"feed_id": "f-swap", "replace_url": "https://example.com/private.zip"},
                {"feed_id": "f-half", "username": "u"}]"#,
        )
        .unwrap();
        assert_eq!(
            config.secret("f-basic").unwrap().basic_auth(),
            Some(("u", "p"))
        );
        assert_eq!(
            config.secret("f-swap").unwrap().replace_url(),
            Some("https://example.com/private.zip")
        );
        // A username with no password satisfies nothing.
        assert_eq!(config.secret("f-half").unwrap().basic_auth(), None);
        assert!(!config.is_empty());
    }

    #[test]
    fn unknown_fields_do_not_stop_the_file_loading() {
        let config = FeedConfig::parse(
            r#"[{"feed_id": "f-a", "key": "k", "aws_region": "us-east-1", "filename": "x.json"}]"#,
        )
        .unwrap();
        assert_eq!(config.get("f-a"), Some("k".to_owned()));
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
            authenticated("f-9q8y-sfmta", "query_param", Some("api_key"), None),
            authenticated(
                "f-sf~bay~area~rg",
                "header",
                Some("Authorization"),
                Some("https://511.org/open-data/token"),
            ),
        ];
        let refs: Vec<&Feed> = feeds.iter().collect();
        let text = template(&refs).unwrap();

        let parsed = FeedConfig::parse(&text).unwrap();
        assert!(parsed.secret("f-9q8y-sfmta").is_some());
        assert!(parsed.secret("f-sf~bay~area~rg").is_some());
        // A template holds no credentials, only the shape of them.
        assert!(parsed.is_empty());

        let guidance = template_guidance(&refs);
        assert!(guidance.contains("query_param"), "{guidance}");
        assert!(
            guidance.contains("https://511.org/open-data/token"),
            "{guidance}"
        );
    }

    #[test]
    fn a_template_with_nothing_to_ask_for_is_an_empty_array() {
        let text = template(&[]).unwrap();
        assert!(FeedConfig::parse(&text).unwrap().is_empty());
        assert!(text.contains("[]"), "{text}");
        assert!(template_guidance(&[]).is_empty());
    }
}
