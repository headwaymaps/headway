//! Credentials for feeds that won't serve their GTFS without one.
//!
//! The atlas says *how* a feed authenticates (`authorization.type`) but never
//! with what, so the value is supplied out of band: a YAML table keyed by the
//! Onestop ID as the atlas spells it, rather than an env-file that would push
//! the shell-safe-name mangling onto whoever fills it in.
//!
//! ```yaml
//! feeds:
//!   f-9q8y-sfmta: "your-token-here"
//!   f-sf~bay~area~rg: "another-token"
//! ```
//!
//! Nothing here relates to the `${VAR}` placeholders in a generated
//! `router-config.json`: those are substituted by OpenTripPlanner at runtime
//! from its own environment (see [`crate::api_keys`]), a deployment concern
//! rather than a build input.
//!
//! [`template`] writes the file for you, from the feeds that still need one.

use crate::dmfr::Feed;
use crate::Result;

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The parsed `--config` file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FeedConfig {
    /// Onestop ID to credential. An empty value counts as absent, so an unfilled
    /// template entry doesn't read as "authenticated with the empty string" -
    /// which servers answer with a 401 that's then hard to explain.
    #[serde(default)]
    pub feeds: BTreeMap<String, String>,
}

impl FeedConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("reading feed config from {}: {e}", path.display()))?;
        Self::parse(&contents).map_err(|e| format!("parsing {}: {e}", path.display()).into())
    }

    fn parse(contents: &str) -> Result<Self> {
        // An empty file deserializes to null rather than an empty map.
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }
        Ok(serde_norway::from_str(contents)?)
    }

    /// The credential for a feed, if we have a non-empty one.
    pub fn get(&self, feed_id: &str) -> Option<String> {
        self.feeds
            .get(feed_id)
            .filter(|value| !value.trim().is_empty())
            .cloned()
    }

    /// Whether anything is actually filled in.
    pub fn is_empty(&self) -> bool {
        self.feeds.values().all(|v| v.trim().is_empty())
    }
}

/// Builds the template listing feeds that still need a credential.
///
/// Hand-written rather than serialized because the useful part is the comments -
/// which agency, how it authenticates, where to ask for a token - and a
/// serializer would leave a wall of bare IDs. `existing` carries over what's
/// already filled in, so regenerating never costs you collected credentials.
pub fn template(needing_credentials: &[&Feed], existing: &FeedConfig) -> String {
    let mut out = String::new();
    out.push_str(
        "# Credentials for GTFS feeds that require one.\n\
         #\n\
         # Generated from feeds the atlas says need authentication and that the\n\
         # index has no extent for. Fill in the values you have, delete the rest,\n\
         # then re-run:\n\
         #\n\
         #   write-gtfs-index --config <this file> --retry-failed\n\
         #\n\
         # --retry-failed matters: failures are remembered so a dead server isn't\n\
         # retried every run, and without it a newly supplied credential has no\n\
         # effect.\n\
         #\n\
         # Treat this file as secret and keep it out of git.\n\
         \n\
         feeds:\n",
    );

    if needing_credentials.is_empty() {
        out.push_str("  # Nothing needs a credential right now.\n");
        // What's already collected still belongs in the file: those feeds
        // presumably measured fine *because* of it.
        for (id, value) in &existing.feeds {
            out.push_str(&format!("  {}: {}\n", quote(id), quote(value)));
        }
        return out;
    }

    for feed in needing_credentials {
        let name = feed.display_name();
        if name != feed.id {
            out.push_str(&format!("\n  # {name}\n"));
        } else {
            out.push('\n');
        }

        if let Some(auth) = &feed.authorization {
            match &auth.param_name {
                Some(param) => {
                    out.push_str(&format!("  # sent as {} {param:?}\n", auth.kind));
                }
                None => out.push_str(&format!("  # sent as {}\n", auth.kind)),
            }
            if let Some(info_url) = &auth.info_url {
                out.push_str(&format!("  # request one at: {info_url}\n"));
            }
        }

        let value = existing.get(&feed.id).unwrap_or_default();
        out.push_str(&format!("  {}: {}\n", quote(&feed.id), quote(&value)));
    }

    // Or regenerating quietly discards working tokens.
    let carried: Vec<(&String, &String)> = existing
        .feeds
        .iter()
        .filter(|(id, value)| {
            !value.trim().is_empty() && !needing_credentials.iter().any(|f| &f.id == *id)
        })
        .collect();

    if !carried.is_empty() {
        out.push_str("\n  # Already supplied, and not currently failing.\n");
        for (id, value) in carried {
            out.push_str(&format!("  {}: {}\n", quote(id), quote(value)));
        }
    }

    out
}

/// Quotes a YAML scalar unconditionally: Onestop IDs are full of `~`, which
/// YAML reads as null, and credentials can be anything at all.
fn quote(value: &str) -> String {
    format!("{:?}", value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmfr::{Authorization, Feed, Operator, Urls};

    use std::collections::BTreeMap as Map;

    fn feed(id: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            spec: "gtfs".to_owned(),
            urls: Urls::default(),
            operators: vec![],
            tags: Map::new(),
            authorization: None,
            source_domain: "example.com".to_owned(),
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
    fn reads_a_table_of_credentials() {
        let config = FeedConfig::parse(
            "feeds:\n  \"f-9q8y-sfmta\": \"secret123\"\n  \"f-sf~bay~area~rg\": \"other\"\n",
        )
        .unwrap();
        assert_eq!(config.get("f-9q8y-sfmta"), Some("secret123".to_owned()));
        assert_eq!(config.get("f-sf~bay~area~rg"), Some("other".to_owned()));
        assert_eq!(config.get("f-nonexistent"), None);
    }

    #[test]
    fn an_empty_value_is_not_a_credential() {
        // The state a freshly generated template is in. Treating "" as a
        // credential would send an empty key and get a 401 back.
        let config = FeedConfig::parse("feeds:\n  \"f-a\": \"\"\n  \"f-b\": \"  \"\n").unwrap();
        assert_eq!(config.get("f-a"), None);
        assert_eq!(config.get("f-b"), None);
        assert!(config.is_empty());
    }

    #[test]
    fn an_empty_file_is_an_empty_config_not_an_error() {
        assert!(FeedConfig::parse("").unwrap().feeds.is_empty());
        assert!(FeedConfig::parse("\n# just a comment\n")
            .unwrap()
            .feeds
            .is_empty());
    }

    #[test]
    fn malformed_yaml_is_an_error() {
        assert!(FeedConfig::parse("feeds: [this is not a map]").is_err());
    }

    #[test]
    fn a_generated_template_round_trips() {
        // The whole point: what we write has to be what we can read back.
        let mut sfmta = authenticated(
            "f-9q8y-sfmta",
            "query_param",
            Some("api_key"),
            Some("https://511.org/open-data/token"),
        );
        sfmta.operators.push(Operator {
            onestop_id: "o-9q8y-sfmta".to_owned(),
            name: Some("San Francisco Municipal Transportation Agency".to_owned()),
            short_name: None,
            website: None,
            associated_feeds: vec![],
            tags: Map::new(),
        });

        let text = template(&[&sfmta], &FeedConfig::default());
        assert!(
            text.contains("San Francisco Municipal Transportation Agency"),
            "{text}"
        );
        assert!(text.contains("query_param"), "{text}");
        assert!(text.contains("https://511.org/open-data/token"), "{text}");

        let parsed = FeedConfig::parse(&text).unwrap();
        assert!(parsed.feeds.contains_key("f-9q8y-sfmta"));
        assert_eq!(parsed.get("f-9q8y-sfmta"), None);
    }

    #[test]
    fn a_tilde_id_survives_the_round_trip() {
        // Bare `~` is null in YAML, and Onestop IDs are full of them.
        let feed = authenticated("f-sf~bay~area~rg", "replace_url", None, None);
        let parsed = FeedConfig::parse(&template(&[&feed], &FeedConfig::default())).unwrap();
        assert!(
            parsed.feeds.contains_key("f-sf~bay~area~rg"),
            "{:?}",
            parsed.feeds
        );
    }

    #[test]
    fn regenerating_keeps_credentials_already_filled_in() {
        let still_failing = authenticated("f-a", "query_param", Some("k"), None);

        let mut existing = FeedConfig::default();
        existing.feeds.insert("f-a".to_owned(), "kept".to_owned());
        // Not in the failing list any more - presumably because this key works.
        existing
            .feeds
            .insert("f-b".to_owned(), "also-kept".to_owned());

        let parsed = FeedConfig::parse(&template(&[&still_failing], &existing)).unwrap();
        assert_eq!(parsed.get("f-a"), Some("kept".to_owned()));
        assert_eq!(parsed.get("f-b"), Some("also-kept".to_owned()));
    }

    #[test]
    fn a_template_with_nothing_to_ask_for_is_still_valid_yaml() {
        let mut existing = FeedConfig::default();
        existing.feeds.insert("f-a".to_owned(), "kept".to_owned());

        let text = template(&[], &existing);
        let parsed = FeedConfig::parse(&text).unwrap();
        assert_eq!(parsed.get("f-a"), Some("kept".to_owned()));

        // And with nothing at all, `feeds:` alone must still parse.
        let empty = FeedConfig::parse(&template(&[], &FeedConfig::default())).unwrap();
        assert!(empty.feeds.is_empty());
    }

    #[test]
    fn a_credential_with_awkward_characters_survives() {
        // Real tokens in the wild contain quotes, colons and hashes.
        let mut existing = FeedConfig::default();
        existing.feeds.insert(
            "f-a".to_owned(),
            r#"tok"en: with #hash \and backslash"#.to_owned(),
        );

        let parsed = FeedConfig::parse(&template(&[], &existing)).unwrap();
        assert_eq!(
            parsed.get("f-a"),
            Some(r#"tok"en: with #hash \and backslash"#.to_owned())
        );
    }
}
