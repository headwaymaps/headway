//! Measuring where a feed actually is, by downloading it and reading its stops.
//!
//! This is the ground truth the whole discovery step rests on. The atlas has no
//! bounding boxes, and the geohash in a Onestop ID is a focal point rather than
//! an extent (see [`crate::geohash`]), so the only way to know whether a feed
//! serves an area is to look at its stops.
//!
//! We read `stops.txt` rather than `shapes.txt` - which is what the `gtfs-bbox`
//! binary uses - because `shapes.txt` is optional in GTFS while `stops.txt` is
//! required. A feed with no shapes still has a knowable extent.
//!
//! Results are stored in a GeoPackage - see [`crate::extents`] - so a re-run
//! costs nothing and an interrupted run resumes where it left off. Failures are
//! recorded too: an agency whose server is down shouldn't be retried on every
//! subsequent run.

use crate::dmfr::Feed;
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
    Measured { bbox: Rect },
    /// We couldn't measure it. Recorded rather than discarded, because a dead
    /// URL is not evidence about geography - discovery reports these so a
    /// curator knows what it couldn't answer for.
    Failed { error: String },
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
    /// Bytes downloaded. Non-zero even when the measurement failed - an HTML
    /// error page served with a 200 still costs bandwidth. Zero when the request
    /// itself failed: reqwest hands us a whole body or an error, never a partial.
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
    // A query_param credential lands in the URL, which reqwest quotes in its
    // errors - so a 401 would write the caller's own token into the index, and
    // out again: the picker can only offer feeds the index could measure.
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
        // Substring, not whole-token: it arrives embedded in a URL.
        Some(secret) if !secret.is_empty() && text.contains(secret) => {
            text.replace(secret, "[redacted]")
        }
        _ => text,
    }
}

/// How to authenticate to a feed URL.
///
/// Mirrors the DMFR `authorization` block, but decoupled from it so the
/// downloader can build one from a CSV row without carrying the whole record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    /// `query_param` or `header`.
    pub kind: String,
    pub param_name: Option<String>,
}

impl From<&crate::dmfr::Authorization> for Auth {
    fn from(auth: &crate::dmfr::Authorization) -> Self {
        Self {
            kind: auth.kind.clone(),
            param_name: auth.param_name.clone(),
        }
    }
}

/// Fetches a feed zip, applying whatever authentication it needs.
///
/// Shared by discovery and by the per-zone download, so an authenticated feed
/// that can be measured can also be built.
pub fn fetch_feed(
    client: &reqwest::blocking::Client,
    url: &str,
    feed_id: &str,
    auth: Option<&Auth>,
    config: &FeedConfig,
) -> Result<Vec<u8>> {
    let mut request = client.get(url).timeout(DOWNLOAD_TIMEOUT);

    if let Some(auth) = auth {
        let Some(key) = config.get(feed_id) else {
            return Err(format!(
                "needs a credential; add {feed_id:?} to the feeds table in your --config \
                 (--write-config-template generates one)"
            )
            .into());
        };

        match auth.kind.as_str() {
            "query_param" => {
                let name = auth.param_name.as_deref().unwrap_or("api_key");
                request = request.query(&[(name, key)]);
            }
            "header" => {
                let name = auth.param_name.as_deref().unwrap_or("Authorization");
                request = request.header(name, key);
            }
            // The catalogued URL is a placeholder; the real one comes from the
            // provider along with your credentials. Only one feed in the atlas
            // uses this - the SF Bay Area 511 regional feed.
            //
            // We accept either form of secret. A full URL replaces the
            // catalogued one outright, which is what the spec describes. A bare
            // token is appended as a query parameter instead, because that's
            // what providers actually hand you: 511 issues a token and
            // documents `?api_key=`, not a personalised URL.
            "replace_url" => {
                if key.starts_with("http://") || key.starts_with("https://") {
                    request = client.get(&key).timeout(DOWNLOAD_TIMEOUT);
                } else {
                    let name = auth.param_name.as_deref().unwrap_or("api_key");
                    request = request.query(&[(name, key)]);
                }
            }
            kind => {
                return Err(format!("unsupported authorization type {kind:?}").into());
            }
        }
    }

    let response = request.send()?.error_for_status()?;
    Ok(response.bytes()?.to_vec())
}

/// Reads `stops.txt` out of a GTFS zip and returns the extent of its stops.
pub fn bbox_from_gtfs_zip(zip_bytes: &[u8]) -> Result<Rect> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        // A 200 response is not proof of a feed: plenty of hosts serve an HTML
        // error page or a JSON "no such object" body with a success status.
        let preview: String = String::from_utf8_lossy(&zip_bytes[..zip_bytes.len().min(120)])
            .chars()
            .filter(|c| !c.is_control())
            .collect();
        format!("not a zip archive ({e}): {preview}")
    })?;

    // GTFS zips occasionally nest the feed inside a directory.
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
    // Trim fields before parsing. Padding a coordinate out to a fixed width is
    // common enough in real feeds - Golden Gate Transit writes "  37.790097" -
    // and Rust's float parser rejects the leading space. Without this, those
    // rows aren't merely awkward, they're unreadable.
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(contents.as_bytes());
    let mut bbox: Option<Rect> = None;
    let mut unreadable = 0;

    for result in reader.deserialize() {
        // One bad row shouldn't cost us the whole feed. Golden Gate Transit
        // used to fail outright on a single unparseable field, taking a network
        // of thousands of stops with it.
        let record: StopRecord = match result {
            Ok(record) => record,
            Err(e) => {
                unreadable += 1;
                log::warn!("skipping unreadable stops.txt row: {e}");
                continue;
            }
        };
        // stop_lat/stop_lon are optional for location types like station
        // entrances and generic nodes, and some feeds leave them blank.
        let (Some(lat), Some(lon)) = (record.stop_lat, record.stop_lon) else {
            continue;
        };
        // 0,0 is the classic "coordinates missing" sentinel. Trusting it drags
        // the box to the Gulf of Guinea and makes the feed match everything in
        // between.
        if lat == 0.0 && lon == 0.0 {
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
        // Distinguish "this feed has no coordinates" from "we couldn't read
        // any of them", which are very different problems to go and look at.
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
        // The real shape: 511 answers a bad key with a 401, and reqwest quotes
        // the whole URL - api_key and all - in the error it hands back.
        let error = "HTTP status client error (401 Unauthorized) for url \
                     (http://api.511.org/transit/datafeeds?operator_id=SF&api_key=s3cret)"
            .to_owned();

        let redacted = redact(error, Some("s3cret"));
        assert!(!redacted.contains("s3cret"), "{redacted}");
        assert!(redacted.contains("[redacted]"), "{redacted}");
        // Everything else survives, or the error stops being diagnosable.
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
    fn bbox_from_stops() {
        let csv = "stop_id,stop_name,stop_lat,stop_lon\n\
                   1,A,47.6,-122.3\n\
                   2,B,47.7,-122.1\n";
        let bbox = bbox_from_stops_csv(csv).unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.3, y: 47.6 });
        assert_eq!(bbox.max(), coord! { x: -122.1, y: 47.7 });
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
        // Golden Gate Transit pads coordinates out to a fixed width. These are
        // perfectly good numbers, so they must be read, not skipped.
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
        // Distinct from "no coordinates": it points at a parsing problem
        // rather than a feed that genuinely has none.
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
