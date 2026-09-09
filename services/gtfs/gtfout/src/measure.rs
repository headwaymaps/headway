//! Measuring where a feed actually is, by downloading it and reading its stops.

use crate::atlas::dmfr::Feed;
use crate::feed_config::FeedConfig;
use crate::geom::RectExt;
use crate::Result;

use geo::{coord, Rect};

use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Some agency servers regenerate the zip on demand, so be generous.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// What we learned about one feed's location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Measurement {
    /// The feed was fetched and its stops span this box.
    Measured {
        bbox: Rect,
    },
    Failed {
        error: String,
    },
}

impl Measurement {
    pub fn bbox(&self) -> Option<Rect> {
        match self {
            Measurement::Measured { bbox } => Some(*bbox),
            Measurement::Failed { .. } => None,
        }
    }
}

/// One feed's result, plus what it cost to get. The byte count drives progress
/// reporting over a run of thousands of downloads.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub measurement: Measurement,
    pub bytes: u64,
}

/// Fetches a feed and measures the extent of its stops.
pub fn measure(
    client: &reqwest::blocking::Client,
    feed: &Feed,
    url: &str,
    config: &FeedConfig,
) -> Outcome {
    let auth = feed.authorization.as_ref().map(Auth::from);
    let secret = auth.as_ref().and_then(|_| config.get(&feed.id));

    let body = match fetch_feed(client, url, &feed.id, auth.as_ref(), config) {
        Ok(body) => body,
        Err(e) => {
            return Outcome {
                measurement: Measurement::Failed {
                    error: redact(e.to_string(), secret.as_deref()),
                },
                bytes: 0,
            }
        }
    };

    let bytes = body.len() as u64;
    let measurement = match bbox_from_gtfs_zip(&body) {
        Ok(bbox) => Measurement::Measured { bbox },
        Err(e) => Measurement::Failed {
            error: redact(e.to_string(), secret.as_deref()),
        },
    };

    Outcome { measurement, bytes }
}

/// Removes a credential from text that's about to be stored or printed.
fn redact(text: String, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() && text.contains(secret) => {
            text.replace(secret, "[redacted]")
        }
        _ => text,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    /// `query_param` or `header`.
    pub kind: String,
    pub param_name: Option<String>,
}

impl From<&crate::atlas::dmfr::Authorization> for Auth {
    fn from(auth: &crate::atlas::dmfr::Authorization) -> Self {
        Self {
            kind: auth.kind.clone(),
            param_name: auth.param_name.clone(),
        }
    }
}

/// Fetches a feed zip, applying whatever authentication it needs.
pub fn fetch_feed(
    client: &reqwest::blocking::Client,
    url: &str,
    feed_id: &str,
    auth: Option<&Auth>,
    config: &FeedConfig,
) -> Result<Vec<u8>> {
    let response = authenticated_request(client, url, feed_id, auth, config)?
        .send()
        .map_err(reqwest::Error::without_url)?
        .error_for_status()
        .map_err(reqwest::Error::without_url)?;
    Ok(response
        .bytes()
        .map_err(reqwest::Error::without_url)?
        .to_vec())
}

/// Verifies that a feed endpoint accepts its configured credential.
pub fn verify_feed(
    client: &reqwest::blocking::Client,
    feed: &Feed,
    url: &str,
    config: &FeedConfig,
) -> Result<()> {
    let auth = feed.authorization.as_ref().map(Auth::from);
    let secret = auth.as_ref().and_then(|_| config.get(&feed.id));

    authenticated_request(client, url, &feed.id, auth.as_ref(), config)
        .and_then(|request| {
            Ok(request
                .send()
                .map_err(reqwest::Error::without_url)?
                .error_for_status()
                .map_err(reqwest::Error::without_url)?)
        })
        .map(|_| ())
        .map_err(|error| redact(error.to_string(), secret.as_deref()).into())
}

fn authenticated_request(
    client: &reqwest::blocking::Client,
    url: &str,
    feed_id: &str,
    auth: Option<&Auth>,
    config: &FeedConfig,
) -> Result<reqwest::blocking::RequestBuilder> {
    let mut request = client.get(url).timeout(DOWNLOAD_TIMEOUT);

    if let Some(auth) = auth {
        let Some(secret) = config.secret(feed_id) else {
            return Err(format!(
                "needs a credential; add an entry for {feed_id:?} to gtfs-secrets.json \
                 (--write-config-template generates the skeleton)"
            )
            .into());
        };

        // A replace_url swaps the url out whatever the declared scheme is, the
        // way transitland's MatchSecrets does, so it is honored before the rest.
        if let Some(replacement) = secret.replace_url() {
            request = client.get(replacement).timeout(DOWNLOAD_TIMEOUT);
        }

        let needs_key = || -> Result<&str> {
            secret.key().ok_or_else(|| {
                format!("gtfs-secrets.json entry for {feed_id:?} has no \"key\"").into()
            })
        };

        match auth.kind.as_str() {
            "query_param" => {
                let name = auth.param_name.as_deref().unwrap_or("api_key");
                request = request.query(&[(name, needs_key()?)]);
            }
            "header" => {
                let name = auth.param_name.as_deref().unwrap_or("Authorization");
                request = request.header(name, needs_key()?);
            }
            "basic_auth" => {
                let Some((user, password)) = secret.basic_auth() else {
                    return Err(format!(
                        "gtfs-secrets.json entry for {feed_id:?} needs \"username\" and \"password\""
                    )
                    .into());
                };
                request = request.basic_auth(user, Some(password));
            }
            "replace_url" => {
                // The url itself is the credential. Before gtfs-secrets.json could
                // express one, a token was smuggled in here and appended as a
                // query param; that still works when no replace_url is set.
                if secret.replace_url().is_none() {
                    let name = auth.param_name.as_deref().unwrap_or("api_key");
                    request = request.query(&[(name, replace_url_token(needs_key()?)?)]);
                }
            }
            kind => {
                return Err(format!("unsupported authorization type {kind:?}").into());
            }
        }
    }

    Ok(request)
}

pub(crate) fn replace_url_token(value: &str) -> Result<&str> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Err(
            "replace_url credentials must be API tokens, not replacement URLs; \
             OpenTripPlanner uses the same value for GTFS-RT"
                .into(),
        );
    }
    Ok(value)
}

/// Reads `stops.txt` out of a GTFS zip and returns the extent of its stops.
pub fn bbox_from_gtfs_zip(zip_bytes: &[u8]) -> Result<Rect> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        let preview: String = String::from_utf8_lossy(&zip_bytes[..zip_bytes.len().min(120)])
            .chars()
            .filter(|c| !c.is_control())
            .collect();
        format!("not a zip archive ({e}): {preview}")
    })?;

    let stops_name = archive
        .file_names()
        .find(|name| {
            name.rsplit('/')
                .next()
                .is_some_and(|base| base.eq_ignore_ascii_case("stops.txt"))
        })
        .map(str::to_owned)
        .ok_or("no stops.txt in archive")?;

    let mut contents = String::new();
    archive
        .by_name(&stops_name)?
        .read_to_string(&mut contents)?;

    bbox_from_stops_csv(&contents)
}

#[derive(Debug, Deserialize)]
struct StopRecord {
    stop_lat: Option<f64>,
    stop_lon: Option<f64>,
}

/// Computes the extent of the stops in a `stops.txt` body.
pub fn bbox_from_stops_csv(contents: &str) -> Result<Rect> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(contents.as_bytes());
    let mut bbox: Option<Rect> = None;
    let mut unreadable = 0;

    for result in reader.deserialize() {
        let record: StopRecord = match result {
            Ok(record) => record,
            Err(e) => {
                unreadable += 1;
                log::warn!("skipping unreadable stops.txt row: {e}");
                continue;
            }
        };
        let (Some(lat), Some(lon)) = (record.stop_lat, record.stop_lon) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
            || (lat == 0.0 && lon == 0.0)
        {
            continue;
        }

        let point = coord! { x: lon, y: lat };
        match &mut bbox {
            Some(bbox) => bbox.expand(point),
            None => bbox = Some(Rect::new(point, point)),
        }
    }

    if unreadable > 0 {
        log::warn!("skipped {unreadable} unreadable rows in stops.txt");
    }

    bbox.ok_or_else(|| {
        if unreadable > 0 {
            format!("every row in stops.txt was unreadable ({unreadable} rows)").into()
        } else {
            crate::Error::from("stops.txt has no usable coordinates")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_credential_is_stripped_from_a_stored_error() {
        let error = "HTTP status client error (401 Unauthorized) for url \
                     (http://api.511.org/transit/datafeeds?operator_id=SF&api_key=s3cret)"
            .to_owned();

        let redacted = redact(error, Some("s3cret"));
        assert!(!redacted.contains("s3cret"), "{redacted}");
        assert!(redacted.contains("[redacted]"), "{redacted}");
        assert!(redacted.contains("401 Unauthorized"), "{redacted}");
        assert!(redacted.contains("api.511.org"), "{redacted}");
    }

    #[test]
    fn redaction_leaves_an_unauthenticated_error_alone() {
        let error = "connection refused".to_owned();
        assert_eq!(redact(error.clone(), None), error);
        assert_eq!(redact(error.clone(), Some("")), error);
    }

    #[test]
    fn replace_url_credentials_must_be_tokens() {
        assert_eq!(replace_url_token("token").unwrap(), "token");
        assert!(replace_url_token("https://example.com/feed.zip").is_err());
    }

    #[test]
    fn ignores_blank_coordinates() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   1,A,47.6,-122.3\n\
                   2,Entrance,,\n\
                   3,B,47.7,-122.1\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.3, y: 47.6 });
        assert_eq!(bbox.max(), coord! { x: -122.1, y: 47.7 });
    }

    #[test]
    fn ignores_nonfinite_and_out_of_range_coordinates() {
        let csv = "stop_lat,stop_lon\nNaN,1\n1,inf\n91,1\n1,-181\n47.6,-122.3\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.3, y: 47.6 });
        assert_eq!(bbox.max(), bbox.min());
    }

    #[test]
    fn ignores_null_island() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   1,A,47.6,-122.3\n\
                   2,Broken,0.0,0.0\n\
                   3,B,47.7,-122.1\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.3, y: 47.6 });
        assert_eq!(bbox.max(), coord! { x: -122.1, y: 47.7 });
    }

    #[test]
    fn reads_whitespace_padded_coordinates() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   40003,Salesforce Transit Center,  37.790097,-122.396066\n\
                   40006,Folsom St & 2nd St,  37.785447,-122.396745\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.396745, y: 37.785447 });
        assert_eq!(bbox.max(), coord! { x: -122.396066, y: 37.790097 });
    }

    #[test]
    fn one_unreadable_row_does_not_lose_the_feed() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   1,A,47.6,-122.3\n\
                   2,Broken,not-a-number,-122.2\n\
                   3,B,47.7,-122.1\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.3, y: 47.6 });
        assert_eq!(bbox.max(), coord! { x: -122.1, y: 47.7 });
    }

    #[test]
    fn an_entirely_unreadable_file_says_so() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   1,A,nope,-122.3\n\
                   2,B,also-nope,-122.1\n";
        let err = bbox_from_stops_csv(csv).unwrap_err().to_string();
        assert!(err.contains("unreadable"), "{err}");
        assert!(err.contains('2'), "should say how many: {err}");
    }

    #[test]
    fn errors_when_there_are_no_coordinates() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n1,A,,\n";
        assert!(bbox_from_stops_csv(csv).is_err());
    }

    #[test]
    fn rejects_a_body_that_isnt_a_zip() {
        let err = bbox_from_gtfs_zip(b"<html><body>404 Not Found</body></html>")
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a zip archive"), "{err}");
        assert!(err.contains("404 Not Found"), "{err}");
    }
}
