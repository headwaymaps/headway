//! Where the vehicles serving a plan's transit legs are right now.
//!
//! Clients poll this against the `patternCode` of the transit legs in a plan they're already
//! showing. Pattern codes are only meaningful to the OTP instance that issued them, so the trip's
//! endpoints come along to pick the same router the plan came from.

use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use chrono::{DateTime, FixedOffset};
use geo::geometry::{LineString, Point};
use polyline::decode_polyline;
use serde::{Deserialize, Serialize};

use super::error::PlanResponseErr;
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql;
use crate::util::bearing_along;
use crate::util::serde_util::deserialize_point_from_lat_lon;
use crate::Error;

/// OTP encodes its polylines at 1e-5, the original Google scale.
const OTP_POLYLINE_PRECISION: u32 = 5;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePositionsQuery {
    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    from_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    to_place: Point,

    /// Comma separated pattern codes, as a plan's transit legs report them.
    patterns: String,
}

impl VehiclePositionsQuery {
    fn pattern_codes(&self) -> Vec<String> {
        self.patterns
            .split(',')
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePositionsResponseOk {
    vehicles: Vec<Vehicle>,
}

/// One transit vehicle's last known position.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    /// Which of the requested patterns this vehicle is serving.
    pattern_code: String,

    /// `FeedId:VehicleId`, unique for as long as the vehicle is reporting.
    #[serde(skip_serializing_if = "Option::is_none")]
    vehicle_id: Option<String>,

    /// What the vehicle shows the public, e.g. a fleet number.
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    lat: f64,
    lon: f64,

    /// Degrees clockwise from north, as the feed reported it. Most feeds don't, so prefer
    /// `bearing`, which is derived rather than published.
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<f64>,

    /// Degrees clockwise from north, taken from the direction the pattern's shape runs where the
    /// vehicle sits on it. Available whether or not the feed publishes a heading of its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    bearing: Option<u16>,

    /// RFC 3339. When the vehicle reported this position.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_updated: Option<DateTime<FixedOffset>>,
}

impl Vehicle {
    /// A vehicle with no coordinates has nothing to draw, so it's dropped rather than represented.
    fn from_otp(
        pattern_code: &str,
        shape: Option<&LineString>,
        position: gtfs_graphql::VehiclePosition,
    ) -> Option<Self> {
        let lat = position.lat?;
        let lon = position.lon?;
        Some(Self {
            pattern_code: pattern_code.to_owned(),
            vehicle_id: position.vehicle_id,
            label: position.label,
            lat,
            lon,
            heading: position.heading,
            bearing: shape.and_then(|shape| bearing_along(shape, Point::new(lon, lat))),
            last_updated: position.last_update,
        })
    }
}

/// The shape a pattern follows, or `None` when OTP has none or it won't decode - in which case
/// its vehicles are still drawn, just without a bearing.
fn shape_of(pattern: &gtfs_graphql::PatternVehicles) -> Option<LineString> {
    let encoded = pattern.geometry.as_ref()?;
    decode_polyline(encoded, OTP_POLYLINE_PRECISION)
        .inspect_err(|e| {
            log::warn!(
                "undecodable shape for pattern {}: {e}",
                pattern.pattern_code
            );
        })
        .ok()
}

impl Responder for VehiclePositionsResponseOk {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> actix_web::HttpResponse {
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        response.content_type("application/json");
        response.json(self)
    }
}

#[get("/v7/vehicle_positions")]
pub async fn get_vehicle_positions(
    query: web::Query<VehiclePositionsQuery>,
    app_state: web::Data<AppState>,
) -> std::result::Result<VehiclePositionsResponseOk, PlanResponseErr> {
    let pattern_codes = query.pattern_codes();
    if pattern_codes.is_empty() {
        return Ok(VehiclePositionsResponseOk { vehicles: vec![] });
    }

    let endpoint = {
        let Some(router) = app_state
            .otp_cluster()
            .find_router(query.from_place, query.to_place)
        else {
            Err(
                Error::user("Transit directions not available for this area.")
                    .error_type(ErrorType::NoCoverageForArea),
            )?
        };
        router.endpoint().clone()
    };

    let client = reqwest::Client::new();
    let vehicles = gtfs_graphql::vehicle_positions(&client, &endpoint, pattern_codes)
        .await
        .map_err(|e| {
            log::error!("error while fetching vehicle positions from otp service: {e}");
            PlanResponseErr::from(e)
        })?;

    let vehicles = vehicles
        .into_iter()
        .flat_map(|pattern| {
            let shape = shape_of(&pattern);
            pattern
                .positions
                .into_iter()
                .filter_map(move |position| {
                    Vehicle::from_otp(&pattern.pattern_code, shape.as_ref(), position)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(VehiclePositionsResponseOk { vehicles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::gtfs_graphql::VehiclePosition;
    use geo::line_string;

    fn position(lat: Option<f64>, lon: Option<f64>) -> VehiclePosition {
        VehiclePosition {
            vehicle_id: Some("1:7204".to_owned()),
            label: Some("7204".to_owned()),
            lat,
            lon,
            heading: None,
            last_update: None,
        }
    }

    /// An eastbound stretch of 1st Ave S.
    fn shape() -> LineString {
        line_string![
            (x: -122.340, y: 47.600),
            (x: -122.330, y: 47.600),
        ]
    }

    fn query(patterns: &str) -> VehiclePositionsQuery {
        VehiclePositionsQuery {
            from_place: Point::new(-122.34, 47.57),
            to_place: Point::new(-122.34, 47.65),
            patterns: patterns.to_owned(),
        }
    }

    #[test]
    fn splits_pattern_codes() {
        assert_eq!(
            query("1:40:0:01,1:21:0:01").pattern_codes(),
            ["1:40:0:01", "1:21:0:01"]
        );
        assert!(query("").pattern_codes().is_empty());
        // A plan whose transit legs all lack a pattern sends an empty element rather than nothing.
        assert_eq!(query("1:40:0:01,").pattern_codes(), ["1:40:0:01"]);
    }

    #[test]
    fn a_vehicle_without_coordinates_is_dropped() {
        let shape = shape();
        let build = |lat, lon| Vehicle::from_otp("1:40:0:01", Some(&shape), position(lat, lon));
        assert!(build(Some(47.6), Some(-122.33)).is_some());
        assert!(build(Some(47.6), None).is_none());
        assert!(build(None, None).is_none());
    }

    #[test]
    fn a_bearing_is_taken_from_the_shape() {
        let vehicle = Vehicle::from_otp(
            "1:40:0:01",
            Some(&shape()),
            position(Some(47.6), Some(-122.335)),
        )
        .expect("has coordinates");
        assert_eq!(vehicle.bearing, Some(89));
    }

    /// OTP has no shape for some patterns. Their vehicles are still worth drawing.
    #[test]
    fn a_pattern_without_a_shape_still_yields_a_vehicle() {
        let vehicle = Vehicle::from_otp("1:40:0:01", None, position(Some(47.6), Some(-122.335)))
            .expect("has coordinates");
        assert_eq!(vehicle.bearing, None);
        assert_eq!(vehicle.lat, 47.6);
    }

    #[test]
    fn an_undecodable_shape_is_dropped_rather_than_failing_the_request() {
        let pattern = gtfs_graphql::PatternVehicles {
            pattern_code: "1:40:0:01".to_owned(),
            geometry: Some("!!! not a polyline !!!".to_owned()),
            positions: vec![],
        };
        assert!(shape_of(&pattern).is_none());
    }
}
