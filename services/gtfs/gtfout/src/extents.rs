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

use crate::measure::Measurement;
use crate::Result;
use geo::{coord, Rect};

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

    /// Forgets every failed measurement, so they'll be retried.
    ///
    /// Failures are cached to stop a dead agency server being hammered on every
    /// run, but that also means supplying a missing API key wouldn't take
    /// effect on its own - the feed would just be answered from the cached
    /// failure. Returns how many were dropped.
    pub fn forget_failures(&self) -> Result<usize> {
        let deleted = self
            .gpkg
            .connection()
            .execute(&format!("DELETE FROM {LAYER} WHERE error IS NOT NULL"), [])?;
        Ok(deleted)
    }

    /// Drops everything known about these feeds. Mostly used by [`Self::insert`]
    /// to make writes replace rather than accumulate.
    pub fn forget(&self, feed_ids: &[String]) -> Result<usize> {
        if feed_ids.is_empty() {
            return Ok(0);
        }

        let mut deleted = 0;
        for feed_id in feed_ids {
            deleted += self.gpkg.connection().execute(
                &format!("DELETE FROM {LAYER} WHERE feed_id = ?1"),
                [feed_id],
            )?;
        }
        Ok(deleted)
    }

    /// Records a batch of measurements, replacing anything already known about
    /// those feeds.
    ///
    /// Two rows for one feed would make every reader here arbitrary:
    /// `extents_by_feed_id` keeps whichever it saw last, and a feed that has
    /// since gone dead goes on matching areas from its stale extent.
    ///
    /// The latest measurement wins even when it's a failure - a feed whose
    /// server now 404s genuinely has no known extent any more.
    pub fn insert(&self, measurements: &[(String, String, Measurement)]) -> Result<()> {
        let feed_ids: Vec<String> = measurements
            .iter()
            .map(|(feed_id, _, _)| feed_id.clone())
            .collect();
        self.forget(&feed_ids)?;

        let layer = self.gpkg.layer(LAYER)?;

        let features: Vec<NewFeature<Polygon>> = measurements
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
        let bbox = BoundingBox::new(area.min().x, area.min().y, area.max().x, area.max().y);

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
            // Note the order: a GeoPackage envelope is [min_x, max_x, min_y,
            // max_y], not the min/max-corner pairing a Rect uses.
            let Some([min_x, max_x, min_y, max_y]) =
                feature.geometry()?.and_then(|g| g.xy_envelope())
            else {
                continue;
            };
            extents.insert(
                feed_id,
                Rect::new(coord! { x: min_x, y: min_y }, coord! { x: max_x, y: max_y }),
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
fn polygon(bbox: &Rect) -> Polygon {
    let (min_x, min_y) = (bbox.min().x, bbox.min().y);
    let (max_x, max_y) = (bbox.max().x, bbox.max().y);

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
            bbox: rect(min_x, min_y, max_x, max_y),
        }
    }

    fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(coord! { x: min_x, y: min_y }, coord! { x: max_x, y: max_y })
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
    fn extents_round_trip_with_axes_the_right_way_round() {
        // A GeoPackage envelope is [min_x, max_x, min_y, max_y], which is not
        // the order a Rect's corners come in. Reading it as corner pairs put
        // longitudes in the latitude columns of the output CSV, and only
        // showed up as a latitude of -120.
        let temp = TempGpkg::new("axes");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-sf~bay~area~rg".to_owned(),
                "https://example.com/rg.zip".to_owned(),
                measured(-123.05377, 36.97492, -120.95269, 39.09981),
            )])
            .unwrap();

        let by_id = extents.extents_by_feed_id().unwrap();
        let bbox = by_id.get("f-sf~bay~area~rg").unwrap();

        assert_eq!(bbox.min(), coord! { x: -123.05377, y: 36.97492 });
        assert_eq!(bbox.max(), coord! { x: -120.95269, y: 39.09981 });

        // The cheap sanity check the original code would have failed: WGS84
        // latitudes live in [-90, 90].
        assert!((-90.0..=90.0).contains(&bbox.min().y));
        assert!((-90.0..=90.0).contains(&bbox.max().y));
    }

    #[test]
    fn forget_failures_clears_only_the_failures() {
        let temp = TempGpkg::new("forget");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[
                (
                    "f-good".to_owned(),
                    "https://example.com/good.zip".to_owned(),
                    measured(-122.4, 47.4, -122.0, 47.8),
                ),
                (
                    "f-sf~bay~area~rg".to_owned(),
                    "https://example.com/rg.zip".to_owned(),
                    Measurement::Failed {
                        error: "needs an API key".to_owned(),
                    },
                ),
            ])
            .unwrap();

        assert_eq!(extents.forget_failures().unwrap(), 1);
        assert!(extents.failures().unwrap().is_empty());

        // The failed one is now unknown, so a re-run will measure it again...
        let keys = extents.measured_keys().unwrap();
        assert!(!keys.contains_key(&(
            "f-sf~bay~area~rg".to_owned(),
            "https://example.com/rg.zip".to_owned()
        )));
        // ...while successful measurements are untouched.
        assert_eq!(
            extents
                .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
                .unwrap(),
            ["f-good"]
        );
    }

    #[test]
    fn re_measuring_a_feed_updates_its_record_rather_than_duplicating_it() {
        let temp = TempGpkg::new("update");
        let extents = FeedExtents::open(&temp.0).unwrap();

        let feed = "f-c23-soundtransit".to_owned();
        let url = "https://example.com/st.zip".to_owned();

        extents
            .insert(&[(
                feed.clone(),
                url.clone(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();
        // Same feed, new extent: the agency moved its service area.
        extents
            .insert(&[(
                feed.clone(),
                url.clone(),
                measured(-123.0, 46.0, -122.5, 46.5),
            )])
            .unwrap();

        // One row, not two...
        let seattle = extents
            .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
            .unwrap();
        assert!(
            seattle.is_empty(),
            "stale extent still matches: {seattle:?}"
        );

        let moved = extents.feeds_in(&rect(-123.1, 45.9, -122.4, 46.6)).unwrap();
        assert_eq!(moved, std::slice::from_ref(&feed));

        // ...and the stored extent is the new one.
        let by_id = extents.extents_by_feed_id().unwrap();
        assert_eq!(by_id.len(), 1);
        assert_eq!(by_id[&feed].min(), coord! { x: -123.0, y: 46.0 });
    }

    #[test]
    fn a_feed_that_has_since_died_stops_matching() {
        // The case the replace exists for: it measured fine once, and now its
        // server 404s. Keeping the old extent would go on recommending a feed
        // the build can no longer download.
        let temp = TempGpkg::new("died");
        let extents = FeedExtents::open(&temp.0).unwrap();

        let feed = "f-gone".to_owned();
        let url = "https://example.com/gone.zip".to_owned();

        extents
            .insert(&[(
                feed.clone(),
                url.clone(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();
        extents
            .insert(&[(
                feed.clone(),
                url,
                Measurement::Failed {
                    error: "404 Not Found".to_owned(),
                },
            )])
            .unwrap();

        assert!(extents
            .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
            .unwrap()
            .is_empty());
        assert_eq!(extents.failures().unwrap().len(), 1);
    }

    #[test]
    fn forgetting_one_feed_leaves_the_others_alone() {
        let temp = TempGpkg::new("forget-one");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[
                (
                    "f-a".to_owned(),
                    "https://example.com/a.zip".to_owned(),
                    measured(-122.4, 47.4, -122.0, 47.8),
                ),
                (
                    "f-b".to_owned(),
                    "https://example.com/b.zip".to_owned(),
                    measured(-122.4, 47.4, -122.0, 47.8),
                ),
            ])
            .unwrap();

        assert_eq!(extents.forget(&["f-a".to_owned()]).unwrap(), 1);
        assert_eq!(
            extents
                .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
                .unwrap(),
            ["f-b"]
        );
        // Forgetting nothing is not an error, and touches nothing.
        assert_eq!(extents.forget(&[]).unwrap(), 0);
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
