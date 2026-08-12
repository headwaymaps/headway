//! Geometry helpers on top of [`geo::Rect`].
//!
//! Everything here works in lon/lat degrees, which is what the atlas, the index
//! and the build scripts all speak.

use geo::{BooleanOps, GeodesicArea, Rect};

/// Parses `"<min_lon> <min_lat> <max_lon> <max_lat>"`, the form the build
/// scripts pass around and the inverse of [`RectExt::bbox_fmt`].
pub fn parse_bbox(s: &str) -> crate::Result<Rect> {
    let values: Vec<f64> = s
        .split_whitespace()
        .map(|v| v.parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| format!("invalid bbox {s:?}: {e}"))?;

    let [min_lon, min_lat, max_lon, max_lat] = values[..] else {
        return Err(format!(
            "bbox needs 4 values (<min_lon> <min_lat> <max_lon> <max_lat>), got {}",
            values.len()
        )
        .into());
    };

    Ok(Rect::new(
        geo::coord! { x: min_lon, y: min_lat },
        geo::coord! { x: max_lon, y: max_lat },
    ))
}

/// What we ask of a bounding box beyond what `geo` already provides.
pub trait RectExt {
    fn area_m2(&self) -> f64;
    fn jaccard(&self, other: &Rect) -> f64;
    fn bbox_fmt(&self) -> String;
    fn expand(&mut self, point: geo::Coord);
}

impl RectExt for Rect {
    /// Area in square meters.
    ///
    /// Degrees squared would be cheaper but isn't comparable across latitudes -
    /// a degree of longitude is 111km at the equator and 43km in Anchorage - so
    /// it makes a northern feed look larger than an equatorial one covering the
    /// same ground.
    fn area_m2(&self) -> f64 {
        self.to_polygon().geodesic_area_unsigned()
    }

    /// How much two areas agree, from 0 (disjoint) to 1 (identical): the shared
    /// area over the area they cover between them.
    ///
    /// Used to rank feeds against a drawn zone. The alternative of subtracting
    /// the spill from the overlap punishes it in absolute terms, which buries
    /// exactly the feeds a zone is built around - a metro operator covering all
    /// of your box and a few times more besides would score worse than a
    /// shuttle serving one building. Measuring the spill against the two areas
    /// keeps that operator on top and still sends a continent-spanning feed to
    /// the bottom, since a rectangle 8,000 times the size of the query can
    /// barely overlap it in proportion.
    fn jaccard(&self, other: &Rect) -> f64 {
        let overlap = self
            .to_polygon()
            .intersection(&other.to_polygon())
            .geodesic_area_unsigned();
        let union = self.area_m2() + other.area_m2() - overlap;

        if union == 0.0 {
            return 0.0;
        }
        overlap / union
    }

    fn bbox_fmt(&self) -> String {
        let (min, max) = (self.min(), self.max());
        format!("{} {} {} {}", min.x, min.y, max.x, max.y)
    }

    /// Grows the rectangle to include a point, if it doesn't already.
    fn expand(&mut self, point: geo::Coord) {
        let (mut min, mut max) = (self.min(), self.max());
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);

        self.set_min(min);
        self.set_max(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use geo::{wkt, Intersects};

    /// A degree box is much smaller in the far north than at the equator, which
    /// is the whole reason this isn't measured in degrees.
    #[test]
    fn area_shrinks_with_latitude() {
        let equator = wkt! { RECT(0. 0.,1. 1.) }.area_m2();
        let anchorage = wkt! { RECT(0. 61.,1. 62.) }.area_m2();

        // A degree square at the equator is ~12,300 km².
        assert!((equator - 12_308e6).abs() / 12_308e6 < 0.01, "{equator} m²");
        assert!(anchorage < equator / 2.0, "{anchorage} vs {equator}");
    }

    /// Rect::new normalizes the corners, so orientation can't flip the sign.
    #[test]
    fn area_is_never_negative() {
        assert_eq!(
            wkt! { RECT(2. 50.,-1. 47.) }.area_m2(),
            wkt! { RECT(-1. 47.,2. 50.) }.area_m2()
        );
    }

    /// Not exactly 1: the intersection is re-tessellated by the boolean op,
    /// which snaps coordinates, so the shared area comes back a fraction of a
    /// part per billion off the original.
    #[test]
    fn identical_rects_score_one() {
        let seattle = wkt! { RECT(-122.462 47.394,-122.005 47.831) };

        assert!(
            (seattle.jaccard(&seattle) - 1.0).abs() < 1e-6,
            "{}",
            seattle.jaccard(&seattle)
        );
    }

    #[test]
    fn disjoint_rects_score_zero() {
        let here = wkt! { RECT(0. 0.,1. 1.) };
        let elsewhere = wkt! { RECT(5. 5.,6. 6.) };

        assert_eq!(here.jaccard(&elsewhere), 0.0);
    }

    /// The ordering the picker relies on: a regional operator covering the
    /// whole query and some more besides beats both a single-building shuttle
    /// inside it and a continental feed that happens to contain it.
    #[test]
    fn ranks_a_regional_operator_over_a_shuttle_and_a_continent() {
        let query = wkt! { RECT(-122.462 47.394,-122.005 47.831) };
        let regional = wkt! { RECT(-122.5 47.1,-121.7 47.9) };
        let shuttle = wkt! { RECT(-122.34 47.6,-122.32 47.62) };
        let continental = wkt! { RECT(-125. 25.,-67. 49.) };

        let (r, s, c) = (
            query.jaccard(&regional),
            query.jaccard(&shuttle),
            query.jaccard(&continental),
        );
        assert!(r > s, "regional {r} should beat shuttle {s}");
        assert!(r > c, "regional {r} should beat continental {c}");
        assert!(s > c, "shuttle {s} should beat continental {c}");
    }

    #[test]
    fn expand_grows_to_fit() {
        let mut bbox = wkt! { RECT(0. 0.,1. 1.) };
        bbox.expand(geo::coord! { x: 5.0, y: -2.0 });

        assert_eq!(bbox, wkt! { RECT(0. -2.,5. 1.) });
    }

    #[test]
    fn overlapping_rects_intersect() {
        assert!(wkt! { RECT(0. 0.,2. 2.) }.intersects(&wkt! { RECT(1. 1.,3. 3.) }));
    }

    #[test]
    fn contained_rect_intersects() {
        assert!(wkt! { RECT(0. 0.,10. 10.) }.intersects(&wkt! { RECT(4. 4.,5. 5.) }));
        assert!(wkt! { RECT(4. 4.,5. 5.) }.intersects(&wkt! { RECT(0. 0.,10. 10.) }));
    }

    #[test]
    fn touching_edges_intersect() {
        assert!(wkt! { RECT(0. 0.,1. 1.) }.intersects(&wkt! { RECT(1. 0.,2. 1.) }));
    }

    #[test]
    fn disjoint_rects_do_not_intersect() {
        // Apart in x only, y only, and both.
        assert!(!wkt! { RECT(0. 0.,1. 1.) }.intersects(&wkt! { RECT(2. 0.,3. 1.) }));
        assert!(!wkt! { RECT(0. 0.,1. 1.) }.intersects(&wkt! { RECT(0. 2.,1. 3.) }));
        assert!(!wkt! { RECT(0. 0.,1. 1.) }.intersects(&wkt! { RECT(5. 5.,6. 6.) }));
    }

    #[test]
    fn parses_a_bbox() {
        let bbox = parse_bbox("-122.462 47.394 -122.005 47.831").unwrap();
        assert_eq!(bbox, wkt! { RECT(-122.462 47.394,-122.005 47.831) });
    }

    #[test]
    fn bbox_parsing_round_trips_through_bbox_fmt() {
        let bbox = wkt! { RECT(-122.462 47.394,-122.005 47.831) };
        assert_eq!(parse_bbox(&bbox.bbox_fmt()).unwrap(), bbox);
    }

    #[test]
    fn rejects_a_malformed_bbox() {
        assert!(parse_bbox("1 2 3").is_err());
        assert!(parse_bbox("1 2 3 4 5").is_err());
        assert!(parse_bbox("north south east west").is_err());
    }
}
