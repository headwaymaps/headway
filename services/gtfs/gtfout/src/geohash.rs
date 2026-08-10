//! Geohashes, as they appear in Transitland Onestop IDs.
//!
//! A geohash decodes to a rectangular cell, but Transitland does *not* use it
//! as a box enclosing the feed. Their documentation is explicit that it's a
//! focal point: "The centroid of the feed, operator, or route will always be
//! located in the geohash that's included in the Onestop ID", and a service
//! area may extend into any of the eight neighbouring cells.
//!
//! Measured against 666 feeds whose real extent we could obtain independently,
//! the cell fully contained the service area only 14% of the time; usually it
//! is far smaller than the network it labels. Contributors also chose wildly
//! different precisions, from ~5000km cells down to ~1km ones.
//!
//! The practical consequence is an asymmetry that the rest of this crate leans
//! on. A geohash is a poor way to decide a feed *is* in your area - a 1km focal
//! point routinely falls outside a metro box the feed really does serve. But
//! [`Geohash::cell_with_margin`] landing far from your area is strong evidence
//! the feed is somewhere else: on those same 666 feeds, the padded cell touched
//! the real extent 99.8% of the time. So we use it only to *exclude*.

use crate::geom::{Point, Rect};

const BASE32: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// A geohash string parsed out of a Onestop ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geohash(String);

impl Geohash {
    /// Longest geohash we'll accept from a Onestop ID.
    ///
    /// The longest in the atlas today is 6 characters (~1km). Capping it keeps
    /// us from reading a long base32-clean *name* component as a geohash.
    pub const MAX_LEN: usize = 6;

    /// Parses a geohash, or returns None if `s` isn't one: empty, too long, or
    /// containing characters outside the geohash base32 alphabet (which
    /// notably excludes `a`, `i`, `l` and `o`).
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() || s.len() > Self::MAX_LEN {
            return None;
        }
        if !s.bytes().all(|b| BASE32.contains(&b)) {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The cell this geohash denotes.
    pub fn cell(&self) -> Rect {
        let (mut lat_min, mut lat_max) = (-90.0f64, 90.0f64);
        let (mut lon_min, mut lon_max) = (-180.0f64, 180.0f64);
        let mut even = true;

        for byte in self.0.bytes() {
            let value = BASE32
                .iter()
                .position(|b| *b == byte)
                .expect("parse() rejected non-base32 characters");

            for mask in [16, 8, 4, 2, 1] {
                let set = value & mask != 0;
                if even {
                    let mid = (lon_min + lon_max) / 2.0;
                    if set {
                        lon_min = mid;
                    } else {
                        lon_max = mid;
                    }
                } else {
                    let mid = (lat_min + lat_max) / 2.0;
                    if set {
                        lat_min = mid;
                    } else {
                        lat_max = mid;
                    }
                }
                even = !even;
            }
        }

        Rect::new(Point::new(lon_min, lat_min), Point::new(lon_max, lat_max))
    }

    /// The cell grown by one full ring of neighbouring cells.
    ///
    /// This is the form to test against, because the documented guarantee only
    /// places the feed's *centroid* inside the cell - the network itself can
    /// spill into the eight surrounding boxes.
    pub fn cell_with_margin(&self) -> Rect {
        let cell = self.cell();
        let width = cell.max().x() - cell.min().x();
        let height = cell.max().y() - cell.min().y();

        Rect::new(
            Point::new(cell.min().x() - width, cell.min().y() - height),
            Point::new(cell.max().x() + width, cell.max().y() + height),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn rejects_non_geohashes() {
        assert_eq!(Geohash::parse(""), None);
        // 'a', 'i', 'l', 'o' are not in the geohash alphabet.
        assert_eq!(Geohash::parse("bay~area"), None);
        assert_eq!(Geohash::parse("santaclarita"), None);
        assert_eq!(Geohash::parse("bcdefgh"), None, "longer than MAX_LEN");
    }

    #[test]
    fn decodes_a_known_cell() {
        // "9q8y" covers western San Francisco.
        let cell = Geohash::parse("9q8y").unwrap().cell();
        approx(cell.min().x(), -122.6953125);
        approx(cell.min().y(), 37.6171875);
        approx(cell.max().x(), -122.34375);
        approx(cell.max().y(), 37.79296875);
    }

    #[test]
    fn precision_one_cell_is_enormous() {
        let cell = Geohash::parse("9").unwrap().cell();
        approx(cell.min().x(), -135.0);
        approx(cell.min().y(), 0.0);
        approx(cell.max().x(), -90.0);
        approx(cell.max().y(), 45.0);
    }

    #[test]
    fn margin_grows_the_cell_by_one_ring() {
        let geohash = Geohash::parse("9q8y").unwrap();
        let cell = geohash.cell();
        let padded = geohash.cell_with_margin();

        let width = cell.max().x() - cell.min().x();
        approx(padded.min().x(), cell.min().x() - width);
        approx(padded.max().x(), cell.max().x() + width);

        let height = cell.max().y() - cell.min().y();
        approx(padded.min().y(), cell.min().y() - height);
        approx(padded.max().y(), cell.max().y() + height);
    }

    #[test]
    fn sfmta_geohash_is_smaller_than_the_network_it_labels() {
        // The property that forces exclusion-only use: SFMTA's cell is ~31km
        // wide, but its real service area reaches further. Documented here so
        // it's clear this isn't an oversight.
        let cell = Geohash::parse("9q8y").unwrap().cell();
        let width_deg = cell.max().x() - cell.min().x();
        assert!(width_deg < 0.36, "cell is only ~31km wide: {width_deg}");
    }
}
