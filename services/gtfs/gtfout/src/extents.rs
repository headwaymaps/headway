//! A GeoPackage of measured feed extents.
//!
//! Discovery has to learn where each feed actually is by downloading it and
//! reading its stops (see [`crate::measure`]). That's expensive enough that we
//! only ever want to do it once per feed, so the results live in a GeoPackage:
//! one polygon per feed, indexed with an RTree.
//!
//! Storing it as a spatial file rather than, say, a pile of JSON is what makes
//! subsequent lookups cheap. Once a feed has been measured, finding every feed
//! serving an area is an indexed spatial query - `feeds_in` below - instead of
//! a scan over thousands of records. Adding a second or third transit zone
//! costs a query, not another download pass.
//!
//! This is a local build artifact. It's derived data, rebuildable by
//! re-downloading, so it belongs in a cache volume rather than in git.
//!
//! # Layout
//!
//! One feature layer, `feed_extents`:
//!
//! | column    | meaning                                              |
//! |-----------|------------------------------------------------------|
//! | geometry  | the feed's extent as a polygon, or NULL if it failed |
//! | `feed_id` | Onestop ID, e.g. `f-c23-soundtransit`                |
//! | `url`     | the URL it was measured from                         |
//! | `error`   | why measurement failed, or NULL on success           |
//!
//! Failures are recorded with a NULL geometry rather than omitted. That keeps
//! an unreachable feed from being retried on every run, and - because a NULL
//! geometry can't match a spatial query - keeps it from being mistaken for a
//! feed that happens to be nearby.

use crate::geom::{Point, Rect};
use crate::measure::Measurement;
use crate::Result;

use std::collections::HashMap;
use std::path::Path;

use geo_types::{Coord, LineString, Polygon};
use geopackage::core::types::{ColumnType, GeometryType};
use geopackage::{
    BoundingBox, ColumnSpec, GeoPackage, GeometrySpec, NewFeature, TableSchemaBuilder, Value,
    ValueRef,
};

const LAYER: &str = "feed_extents";
const WGS84: i32 = 4326;

/// How many rows to write per transaction.
const BATCH_SIZE: usize = 256;

pub struct FeedExtents {
    gpkg: GeoPackage,
}

impl FeedExtents {
    /// Opens the GeoPackage at `path`, creating it if it isn't there yet.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let gpkg = if path.exists() {
            GeoPackage::open(path)?
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let gpkg = GeoPackage::create(path)?;
            // TableSchemaBuilder builds the RTree index by default, which is
            // what keeps feeds_in() from degrading to a full scan.
            gpkg.create_layer(
                &TableSchemaBuilder::new(LAYER)
                    .column(ColumnSpec::new("feed_id", ColumnType::Text(None)))
                    .column(ColumnSpec::new("url", ColumnType::Text(None)))
                    .column(ColumnSpec::new("error", ColumnType::Text(None)))
                    .geometry(GeometrySpec::new(GeometryType::Polygon, WGS84)),
            )?;
            gpkg
        };

        Ok(Self { gpkg })
    }

    /// Every (feed_id, url) pair already measured, so a run knows what it can
    /// skip without querying per feed.
    pub fn measured_keys(&self) -> Result<HashMap<(String, String), bool>> {
        let layer = self.gpkg.layer(LAYER)?;
        let mut keys = HashMap::new();

        for feature in layer.features()? {
            let feature = feature?;
            let Some(feed_id) = feature.value("feed_id").and_then(text) else {
                continue;
            };
            let Some(url) = feature.value("url").and_then(text) else {
                continue;
            };
            let succeeded = feature.geometry_bytes().is_some();
            keys.insert((feed_id, url), succeeded);
        }

        Ok(keys)
    }

    /// Records a batch of measurements.
    pub fn insert(&self, measurements: &[(String, String, Measurement)]) -> Result<()> {
        let layer = self.gpkg.layer(LAYER)?;

        let features: Vec<NewFeature<Polygon<f64>>> = measurements
            .iter()
            .map(|(feed_id, url, measurement)| {
                let values = vec![
                    Value::Text(feed_id.clone()),
                    Value::Text(url.clone()),
                    match measurement {
                        Measurement::Measured { .. } => Value::Null,
                        Measurement::Failed { error } => Value::Text(error.clone()),
                    },
                ];

                match measurement.bbox() {
                    Some(bbox) => NewFeature::new(polygon(&bbox), values),
                    None => NewFeature::attributes(values),
                }
            })
            .collect();

        layer.write_all(features, BATCH_SIZE)?;
        Ok(())
    }

    /// The feeds whose measured extent intersects `area`, as Onestop IDs.
    ///
    /// This is the query the whole file exists for: an RTree lookup rather
    /// than a scan, so asking about another area is nearly free.
    pub fn feeds_in(&self, area: &Rect) -> Result<Vec<String>> {
        let layer = self.gpkg.layer(LAYER)?;
        let bbox = BoundingBox::new(
            area.min().x(),
            area.min().y(),
            area.max().x(),
            area.max().y(),
        );

        let mut feed_ids = Vec::new();
        for feature in layer.features_in(bbox)? {
            let feature = feature?;
            if let Some(feed_id) = feature.value("feed_id").and_then(text) {
                feed_ids.push(feed_id);
            }
        }

        feed_ids.sort();
        feed_ids.dedup();
        Ok(feed_ids)
    }

    /// The measured extent of every successfully measured feed, by Onestop ID.
    ///
    /// Read from the stored envelope rather than by decoding each polygon: the
    /// geometry is a bbox rectangle, so its envelope is exactly the extent.
    pub fn extents_by_feed_id(&self) -> Result<HashMap<String, Rect>> {
        let layer = self.gpkg.layer(LAYER)?;
        let mut extents = HashMap::new();

        for feature in layer.select("error IS NULL", &[])? {
            let feature = feature?;
            let Some(feed_id) = feature.value("feed_id").and_then(text) else {
                continue;
            };
            let Some([min_x, min_y, max_x, max_y]) =
                feature.geometry()?.and_then(|g| g.xy_envelope())
            else {
                continue;
            };
            extents.insert(
                feed_id,
                Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y)),
            );
        }

        Ok(extents)
    }

    /// Feeds we tried and failed to measure, with the reason.
    ///
    /// These are neither in nor out of an area - we simply don't know. They're
    /// reported so a curator can see what discovery couldn't answer for.
    pub fn failures(&self) -> Result<Vec<(String, String, String)>> {
        let layer = self.gpkg.layer(LAYER)?;
        let mut failures = Vec::new();

        for feature in layer.select("error IS NOT NULL", &[])? {
            let feature = feature?;
            let (Some(feed_id), Some(url), Some(error)) = (
                feature.value("feed_id").and_then(text),
                feature.value("url").and_then(text),
                feature.value("error").and_then(text),
            ) else {
                continue;
            };
            failures.push((feed_id, url, error));
        }

        failures.sort();
        Ok(failures)
    }
}

fn text(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Text(s) => Some(s.to_owned()),
        _ => None,
    }
}

/// A bbox as a closed rectangular ring, which is what a GeoPackage polygon
/// wants.
fn polygon(bbox: &Rect) -> Polygon<f64> {
    let Point { .. } = bbox.min();
    let (min_x, min_y) = (bbox.min().x(), bbox.min().y());
    let (max_x, max_y) = (bbox.max().x(), bbox.max().y());

    Polygon::new(
        LineString::new(vec![
            Coord { x: min_x, y: min_y },
            Coord { x: max_x, y: min_y },
            Coord { x: max_x, y: max_y },
            Coord { x: min_x, y: max_y },
            Coord { x: min_x, y: min_y },
        ]),
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempGpkg(std::path::PathBuf);

    impl TempGpkg {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "gtfout-{name}-{}-{:?}.gpkg",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::remove_file(&path).ok();
            Self(path)
        }
    }

    impl Drop for TempGpkg {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).ok();
        }
    }

    fn measured(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Measurement {
        Measurement::Measured {
            min_lon: min_x,
            min_lat: min_y,
            max_lon: max_x,
            max_lat: max_y,
        }
    }

    fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    #[test]
    fn round_trips_a_measurement() {
        let temp = TempGpkg::new("roundtrip");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-c23-soundtransit".to_owned(),
                "https://example.com/st.zip".to_owned(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();

        let keys = extents.measured_keys().unwrap();
        assert_eq!(
            keys.get(&(
                "f-c23-soundtransit".to_owned(),
                "https://example.com/st.zip".to_owned()
            )),
            Some(&true)
        );
    }

    #[test]
    fn spatial_query_finds_overlapping_feeds_only() {
        let temp = TempGpkg::new("spatial");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[
                (
                    "f-c23-soundtransit".to_owned(),
                    "https://example.com/st.zip".to_owned(),
                    measured(-122.4, 47.4, -122.0, 47.8),
                ),
                (
                    "f-9q8y-sfmta".to_owned(),
                    "https://example.com/sf.zip".to_owned(),
                    measured(-122.5, 37.7, -122.3, 37.8),
                ),
            ])
            .unwrap();

        let seattle = extents
            .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
            .unwrap();
        assert_eq!(seattle, ["f-c23-soundtransit"]);

        let bay = extents.feeds_in(&rect(-123.1, 36.9, -121.2, 38.6)).unwrap();
        assert_eq!(bay, ["f-9q8y-sfmta"]);
    }

    #[test]
    fn a_continental_feed_matches_everywhere_it_spans() {
        // Amtrak really does span the country; this isn't a false positive, and
        // the extent columns in the output CSV are what let a curator judge it.
        let temp = TempGpkg::new("continental");
        let extents = FeedExtents::open(&temp.0).unwrap();
        extents
            .insert(&[(
                "f-9-amtrak".to_owned(),
                "https://example.com/amtrak.zip".to_owned(),
                measured(-124.0, 25.0, -68.0, 49.0),
            )])
            .unwrap();

        assert_eq!(
            extents
                .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
                .unwrap(),
            ["f-9-amtrak"]
        );
        assert_eq!(
            extents.feeds_in(&rect(-123.1, 36.9, -121.2, 38.6)).unwrap(),
            ["f-9-amtrak"]
        );
    }

    #[test]
    fn failures_are_recorded_but_never_match_a_query() {
        let temp = TempGpkg::new("failures");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-broken~wa~us".to_owned(),
                "https://example.com/broken.zip".to_owned(),
                Measurement::Failed {
                    error: "connection refused".to_owned(),
                },
            )])
            .unwrap();

        // Remembered, so we don't retry it every run...
        let keys = extents.measured_keys().unwrap();
        assert_eq!(
            keys.get(&(
                "f-broken~wa~us".to_owned(),
                "https://example.com/broken.zip".to_owned()
            )),
            Some(&false)
        );

        // ...reportable...
        let failures = extents.failures().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, "f-broken~wa~us");
        assert_eq!(failures[0].2, "connection refused");

        // ...but never silently treated as being somewhere.
        let anywhere = extents.feeds_in(&rect(-180.0, -90.0, 180.0, 90.0)).unwrap();
        assert!(anywhere.is_empty(), "{anywhere:?}");
    }

    #[test]
    fn reopening_preserves_contents() {
        let temp = TempGpkg::new("reopen");
        {
            let extents = FeedExtents::open(&temp.0).unwrap();
            extents
                .insert(&[(
                    "f-c23-soundtransit".to_owned(),
                    "https://example.com/st.zip".to_owned(),
                    measured(-122.4, 47.4, -122.0, 47.8),
                )])
                .unwrap();
        }

        let reopened = FeedExtents::open(&temp.0).unwrap();
        assert_eq!(
            reopened
                .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
                .unwrap(),
            ["f-c23-soundtransit"]
        );
    }
}
