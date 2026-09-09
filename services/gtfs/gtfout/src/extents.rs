//! A GeoPackage of measured feed extents, and what the atlas says about each.
//!
//! Each row carries a feed's measured geometry alongside the DMFR details a
//! zone file needs - provider name, url, authorization and realtime feeds.
//! Those are written by [`FeedExtents::set_metadata`], from the catalog, and
//! read back by [`FeedExtents::measured_feeds`].

use crate::measure::Measurement;
use crate::transit_zone::zone::ZoneFeed;
use crate::Result;
use geo::{coord, Polygon, Rect};

use std::collections::HashMap;
use std::path::Path;

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
            gpkg.create_layer(
                &TableSchemaBuilder::new(LAYER)
                    .column(ColumnSpec::new("feed_id", ColumnType::Text(None)))
                    .column(ColumnSpec::new("url", ColumnType::Text(None)))
                    .column(ColumnSpec::new("error", ColumnType::Text(None)))
                    .column(ColumnSpec::new("metadata", ColumnType::Text(None)))
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
                    // Filled in by set_metadata once the catalog is to hand.
                    Value::Null,
                ];

                match measurement.bbox() {
                    Some(bbox) => NewFeature::new(bbox.to_polygon(), values),
                    None => NewFeature::attributes(values),
                }
            })
            .collect();

        layer.write_all(features, BATCH_SIZE)?;
        Ok(())
    }

    /// Writes what the atlas says about each feed onto the row that feed
    /// already has. Feeds with no row yet - never measured, or measured on a
    /// run that has not finished - are left for a later pass.
    pub fn set_metadata(&self, metadata: &[(String, ZoneFeed)]) -> Result<usize> {
        let connection = self.gpkg.connection();
        connection.execute_batch("BEGIN")?;

        let mut updated = 0;
        for (feed_id, feed) in metadata {
            let json = serde_json::to_string(feed)?;
            updated += connection.execute(
                &format!("UPDATE {LAYER} SET metadata = ?1 WHERE feed_id = ?2"),
                [json.as_str(), feed_id.as_str()],
            )?;
        }

        connection.execute_batch("COMMIT")?;
        Ok(updated)
    }

    /// Every feed a zone can be built from: measured, and described by the
    /// atlas. A row missing either is not offerable and is left out.
    pub fn measured_feeds(&self) -> Result<Vec<(ZoneFeed, Rect)>> {
        let layer = self.gpkg.layer(LAYER)?;
        let mut feeds = Vec::new();

        for feature in layer.select("error IS NULL AND metadata IS NOT NULL", &[])? {
            let feature = feature?;
            let Some(feed_id) = feature.value("feed_id").and_then(text) else {
                continue;
            };
            let Some(metadata) = feature.value("metadata").and_then(text) else {
                continue;
            };
            let Some([min_x, max_x, min_y, max_y]) =
                feature.geometry()?.and_then(|g| g.xy_envelope())
            else {
                continue;
            };

            let feed: ZoneFeed = serde_json::from_str(&metadata).map_err(|e| {
                format!(
                    "{feed_id}: this index was written by a different gtfout ({e});                      re-run build-gtfs-index"
                )
            })?;
            feeds.push((
                feed,
                Rect::new(coord! { x: min_x, y: min_y }, coord! { x: max_x, y: max_y }),
            ));
        }

        Ok(feeds)
    }

    /// The feeds whose measured extent intersects `area`, as Onestop IDs.
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

    /// Feeds we tried and failed to measure, with the reason.
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

    /// What `build-gtfs-index` bakes in for a feed, reduced to what a test needs.
    fn described(feed_id: &str, provider: &str) -> (String, ZoneFeed) {
        (
            feed_id.to_owned(),
            ZoneFeed {
                feed_onestop_id: feed_id.to_owned(),
                provider: provider.to_owned(),
                url: format!("https://example.com/{feed_id}.zip"),
                authorization: None,
                realtime: vec![],
            },
        )
    }

    /// The measured extents, by feed id, as the tests used to read them.
    fn extents_by_feed_id(extents: &FeedExtents) -> HashMap<String, Rect> {
        extents
            .measured_feeds()
            .unwrap()
            .into_iter()
            .map(|(feed, rect)| (feed.feed_onestop_id, rect))
            .collect()
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

        let keys = extents.measured_keys().unwrap();
        assert_eq!(
            keys.get(&(
                "f-broken~wa~us".to_owned(),
                "https://example.com/broken.zip".to_owned()
            )),
            Some(&false)
        );

        let failures = extents.failures().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, "f-broken~wa~us");
        assert_eq!(failures[0].2, "connection refused");

        let anywhere = extents.feeds_in(&rect(-180.0, -90.0, 180.0, 90.0)).unwrap();
        assert!(anywhere.is_empty(), "{anywhere:?}");
    }

    #[test]
    fn extents_round_trip_with_axes_the_right_way_round() {
        let temp = TempGpkg::new("axes");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-sf~bay~area~rg".to_owned(),
                "https://example.com/rg.zip".to_owned(),
                measured(-123.05377, 36.97492, -120.95269, 39.09981),
            )])
            .unwrap();

        extents
            .set_metadata(&[described("f-sf~bay~area~rg", "511")])
            .unwrap();

        let by_id = extents_by_feed_id(&extents);
        let bbox = by_id.get("f-sf~bay~area~rg").unwrap();

        assert_eq!(bbox.min(), coord! { x: -123.05377, y: 36.97492 });
        assert_eq!(bbox.max(), coord! { x: -120.95269, y: 39.09981 });

        assert!((-90.0..=90.0).contains(&bbox.min().y));
        assert!((-90.0..=90.0).contains(&bbox.max().y));
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
        extents
            .insert(&[(
                feed.clone(),
                url.clone(),
                measured(-123.0, 46.0, -122.5, 46.5),
            )])
            .unwrap();

        let seattle = extents
            .feeds_in(&rect(-122.462, 47.394, -122.005, 47.831))
            .unwrap();
        assert!(
            seattle.is_empty(),
            "stale extent still matches: {seattle:?}"
        );

        let moved = extents.feeds_in(&rect(-123.1, 45.9, -122.4, 46.6)).unwrap();
        assert_eq!(moved, std::slice::from_ref(&feed));

        extents
            .set_metadata(&[described(&feed, "Sound Transit")])
            .unwrap();

        let by_id = extents_by_feed_id(&extents);
        assert_eq!(by_id.len(), 1);
        assert_eq!(by_id[&feed].min(), coord! { x: -123.0, y: 46.0 });
    }

    #[test]
    fn a_feed_that_has_since_died_stops_matching() {
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
    fn the_atlas_details_survive_the_round_trip() {
        let temp = TempGpkg::new("metadata");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-c23-kcm".to_owned(),
                "https://example.com/kcm.zip".to_owned(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();

        let (id, mut feed) = described("f-c23-kcm", "King County Metro");
        feed.realtime.push(crate::transit_zone::zone::ZoneRealtime {
            feed_onestop_id: "f-c23-kcm~rt".to_owned(),
            urls: crate::transit_zone::zone::RealtimeUrls {
                trip_updates: Some("https://example.com/tu.pb".to_owned()),
                ..Default::default()
            },
            authorization: Some(crate::transit_zone::zone::ZoneAuth {
                kind: "query_param".to_owned(),
                param_name: Some("key".to_owned()),
                info_url: Some("https://example.com/keys".to_owned()),
            }),
        });
        assert_eq!(extents.set_metadata(&[(id, feed)]).unwrap(), 1);

        let feeds = extents.measured_feeds().unwrap();
        assert_eq!(feeds.len(), 1);
        let (feed, extent) = &feeds[0];
        assert_eq!(feed.provider, "King County Metro");
        assert_eq!(feed.realtime.len(), 1);
        assert_eq!(feed.realtime[0].feed_onestop_id, "f-c23-kcm~rt");
        assert_eq!(
            feed.realtime[0].authorization.as_ref().unwrap().info_url,
            Some("https://example.com/keys".to_owned())
        );
        assert_eq!(extent.min(), coord! { x: -122.4, y: 47.4 });
    }

    #[test]
    fn a_measured_feed_the_atlas_never_described_is_not_offerable() {
        let temp = TempGpkg::new("undescribed");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-orphan".to_owned(),
                "https://example.com/orphan.zip".to_owned(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();

        // Still found by a spatial query, but nothing can be built from it.
        assert_eq!(
            extents.feeds_in(&rect(-123.0, 47.0, -122.0, 48.0)).unwrap(),
            ["f-orphan"]
        );
        assert!(extents.measured_feeds().unwrap().is_empty());
    }

    #[test]
    fn re_describing_a_feed_replaces_what_the_atlas_used_to_say() {
        let temp = TempGpkg::new("redescribe");
        let extents = FeedExtents::open(&temp.0).unwrap();

        extents
            .insert(&[(
                "f-moved".to_owned(),
                "https://example.com/old.zip".to_owned(),
                measured(-122.4, 47.4, -122.0, 47.8),
            )])
            .unwrap();
        extents
            .set_metadata(&[described("f-moved", "Old Name")])
            .unwrap();

        let (id, mut feed) = described("f-moved", "New Name");
        feed.url = "https://example.com/new.zip".to_owned();
        extents.set_metadata(&[(id, feed)]).unwrap();

        let feeds = extents.measured_feeds().unwrap();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].0.provider, "New Name");
        assert_eq!(feeds[0].0.url, "https://example.com/new.zip");
    }

    #[test]
    fn describing_a_feed_with_no_row_yet_changes_nothing() {
        let temp = TempGpkg::new("unmeasured");
        let extents = FeedExtents::open(&temp.0).unwrap();

        assert_eq!(
            extents
                .set_metadata(&[described("f-never~measured", "Nobody")])
                .unwrap(),
            0
        );
        assert!(extents.measured_feeds().unwrap().is_empty());
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
        assert_eq!(extents.forget(&[]).unwrap(), 0);
    }
}
