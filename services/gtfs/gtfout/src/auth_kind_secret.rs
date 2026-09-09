//! A feed's declared authentication joined to the secret that satisfies it.

use crate::atlas::dmfr::{AuthKind, FeedId};
use crate::feed_config::{Secret, SecretValue};

use std::collections::BTreeMap;

use base64::Engine;

/// What a request actually carries. Built by pairing the feed's [`AuthKind`]
/// with its secret, so a key for a feed wanting basic auth cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKindSecret {
    QueryParam { param_name: String, key: String },
    Header { param_name: String, key: String },
    BasicAuth { username: String, password: String },
}

impl AuthKindSecret {
    pub fn resolve(kind: &AuthKind, secret: &Secret, feed_id: &FeedId) -> Result<Self, String> {
        Ok(match (kind, &secret.value) {
            (AuthKind::QueryParam { param_name }, SecretValue::Key { key }) => Self::QueryParam {
                param_name: param_name.clone(),
                key: key.clone(),
            },
            (AuthKind::Header { param_name }, SecretValue::Key { key }) => Self::Header {
                param_name: param_name.clone(),
                key: key.clone(),
            },
            (AuthKind::BasicAuth, SecretValue::BasicAuth { username, password }) => {
                Self::BasicAuth {
                    username: username.clone(),
                    password: password.clone(),
                }
            }
            (AuthKind::QueryParam { .. } | AuthKind::Header { .. }, SecretValue::BasicAuth { .. }) => {
                return Err(format!(
                    "gtfs-secrets.json entry for {feed_id:?} has a username and password, but this feed wants a \"key\""
                ))
            }
            (AuthKind::BasicAuth, SecretValue::Key { .. }) => {
                return Err(format!(
                    "gtfs-secrets.json entry for {feed_id:?} has a \"key\", but this feed needs \"username\" and \"password\""
                ))
            }
        })
    }

    /// The url to fetch, with whatever this credential adds to it.
    pub fn url(&self, url: &str) -> String {
        match self {
            Self::QueryParam { param_name, key } => format!(
                "{url}{}{param_name}={key}",
                if url.contains('?') { '&' } else { '?' }
            ),
            Self::Header { .. } | Self::BasicAuth { .. } => url.to_owned(),
        }
    }

    pub fn headers(&self) -> Option<BTreeMap<String, String>> {
        let (name, value) = match self {
            Self::QueryParam { .. } => return None,
            Self::Header { param_name, key } => (param_name.clone(), key.clone()),
            Self::BasicAuth { username, password } => {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}:{password}"));
                ("Authorization".to_owned(), format!("Basic {encoded}"))
            }
        };
        Some(BTreeMap::from([(name, value)]))
    }
}
