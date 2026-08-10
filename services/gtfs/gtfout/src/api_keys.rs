//! API keys for feeds that require authentication to download.
//!
//! Some agencies gate their GTFS behind a key. The DMFR record says *how* to
//! authenticate (`authorization.type`), but never the key itself, so we supply
//! it out of band.
//!
//! Keys come from an env-file of `KEY=VALUE` lines rather than the process
//! environment, because these tools run inside dagger, which sandboxes module
//! code and can't forward the host environment. The build mounts the file as a
//! secret; see `GtfsApiKeysPath` in dagger/transit.go. Running the binaries by
//! hand, the process environment works too and needs no flag.

use crate::Result;

use std::collections::HashMap;
use std::path::Path;

/// Env var holding the API key for a feed that needs one.
///
/// Onestop IDs contain characters that aren't legal in shell variable names, so
/// everything but ASCII alphanumerics becomes an underscore:
/// `f-9q8y-sfmta` -> `HEADWAY_GTFS_API_KEY_F_9Q8Y_SFMTA`.
pub fn api_key_env_var(feed_id: &str) -> String {
    let slug: String = feed_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("HEADWAY_GTFS_API_KEY_{slug}")
}

/// Somewhere to look up a feed's API key.
#[derive(Debug, Default)]
pub struct ApiKeys {
    from_file: HashMap<String, String>,
}

impl ApiKeys {
    /// Keys from the process environment only.
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Loads an env-file of `KEY=VALUE` lines.
    ///
    /// The file is shared with other tooling (it's the repo's `.bin-env`), so
    /// unrelated lines, blanks and `#` comments are ignored rather than being
    /// treated as errors. Values may be quoted.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("reading API keys from {}: {e}", path.display()))?;
        Ok(Self::parse(&contents))
    }

    fn parse(contents: &str) -> Self {
        let mut from_file = HashMap::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // `export FOO=bar` is valid in the env-files this reads.
            let line = line.strip_prefix("export ").unwrap_or(line).trim_start();

            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if !name.starts_with("HEADWAY_GTFS_API_KEY_") {
                continue;
            }

            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);

            if !value.is_empty() {
                from_file.insert(name.to_owned(), value.to_owned());
            }
        }

        Self { from_file }
    }

    /// The key for a feed, if we have one.
    ///
    /// The file wins over the process environment: it's the explicit, per-build
    /// input, whereas the environment is whatever happens to be lying around.
    pub fn get(&self, feed_id: &str) -> Option<String> {
        let name = api_key_env_var(feed_id);
        self.from_file
            .get(&name)
            .cloned()
            .or_else(|| std::env::var(&name).ok())
            .filter(|key| !key.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_name_is_shell_safe() {
        assert_eq!(
            api_key_env_var("f-9q8y-sfmta"),
            "HEADWAY_GTFS_API_KEY_F_9Q8Y_SFMTA"
        );
        assert_eq!(
            api_key_env_var("f-sf~bay~area~rg"),
            "HEADWAY_GTFS_API_KEY_F_SF_BAY_AREA_RG"
        );
        // Non-ASCII appears in real Onestop IDs, e.g. f-aléop~pays~de~la~loire.
        assert_eq!(
            api_key_env_var("f-aléop~pays"),
            "HEADWAY_GTFS_API_KEY_F_AL_OP_PAYS"
        );
    }

    #[test]
    fn parses_an_env_file() {
        let keys = ApiKeys::parse(
            "# a comment\n\
             \n\
             HEADWAY_GTFS_API_KEY_F_9Q8Y_SFMTA=secret123\n\
             HEADWAY_OTHER_SETTING=not-a-key\n",
        );
        assert_eq!(keys.get("f-9q8y-sfmta"), Some("secret123".to_owned()));
    }

    #[test]
    fn ignores_unrelated_lines() {
        // The file is the repo's shared .bin-env, so most of it isn't ours.
        let keys = ApiKeys::parse(
            "DEPLOYMENT_ARTIFACT_ROOT=https://example.com\n\
             malformed line with no equals\n\
             HEADWAY_GTFS_API_KEY_F_X=k\n",
        );
        assert_eq!(keys.from_file.len(), 1);
        assert_eq!(keys.get("f-x"), Some("k".to_owned()));
    }

    #[test]
    fn strips_quotes_and_export() {
        let keys = ApiKeys::parse(
            "export HEADWAY_GTFS_API_KEY_F_A=\"quoted\"\n\
             HEADWAY_GTFS_API_KEY_F_B='single'\n",
        );
        assert_eq!(keys.get("f-a"), Some("quoted".to_owned()));
        assert_eq!(keys.get("f-b"), Some("single".to_owned()));
    }

    #[test]
    fn empty_values_are_not_keys() {
        // An unset placeholder shouldn't read as "authenticated with ''".
        let keys = ApiKeys::parse("HEADWAY_GTFS_API_KEY_F_A=\n");
        assert_eq!(keys.get("f-a"), None);
    }

    #[test]
    fn missing_key_is_none() {
        let keys = ApiKeys::parse("HEADWAY_GTFS_API_KEY_F_A=k\n");
        assert_eq!(keys.get("f-nonexistent"), None);
    }
}
