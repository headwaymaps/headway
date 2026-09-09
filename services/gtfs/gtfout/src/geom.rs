//! Geometry helpers on top of [`geo::Rect`].

use geo::{BooleanOps, GeodesicArea, Rect};

/// What we ask of a bounding box beyond what `geo` already provides.
pub trait RectExt {
    fn area_m2(&self) -> f64;
    fn jaccard(&self, other: &Rect) -> f64;
    fn bbox_fmt(&self) -> String;
    fn expand(&mut self, point: impl Into<geo::Coord>);
}

impl RectExt for Rect {
    /// Area in square meters.
    fn area_m2(&self) -> f64 {
        self.to_polygon().geodesic_area_unsigned()
    }

    /// How much two areas agree, from 0 (disjoint) to 1 (identical): the shared
    /// area over the area they cover between them.
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

    fn expand(&mut self, point: impl Into<geo::Coord>) {
        let point = point.into();
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

    use geo::wkt;

    #[test]
    fn area_shrinks_with_latitude() {
        let equator = wkt! { RECT(0. 0.,1. 1.) }.area_m2();
        let anchorage = wkt! { RECT(0. 61.,1. 62.) }.area_m2();

        assert!((equator - 12_308e6).abs() / 12_308e6 < 0.01, "{equator} m²");
        assert!(anchorage < equator / 2.0, "{anchorage} vs {equator}");
    }

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
}
