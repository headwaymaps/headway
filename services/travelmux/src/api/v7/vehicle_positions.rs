//! Where the vehicles serving a plan's transit legs are right now.
//!
//! Clients poll this against the `patternCode` of the transit legs in a plan they're already
//! showing. Pattern codes are only meaningful to the OTP instance that issued them, so the trip's
//! endpoints come along to pick the same router the plan came from.

use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use chrono::{DateTime, FixedOffset};
use geo::geometry::Point;
use serde::{Deserialize, Serialize};

use super::error::PlanResponseErr;
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql;
use crate::util::serde_util::deserialize_point_from_lat_lon;
use crate::Error;

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

    /// Degrees clockwise from north.
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<f64>,

    /// RFC 3339. When the vehicle reported this position.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_updated: Option<DateTime<FixedOffset>>,
}

impl Vehicle {
    /// A vehicle with no coordinates has nothing to draw, so it's dropped rather than represented.
    fn from_otp(vehicle: gtfs_graphql::PatternVehicle) -> Option<Self> {
        let position = vehicle.position;
        Some(Self {
            pattern_code: vehicle.pattern_code,
            vehicle_id: position.vehicle_id,
            label: position.label,
            lat: position.lat?,
            lon: position.lon?,
            heading: position.heading,
            last_updated: position.last_update,
        })
    }
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

    Ok(VehiclePositionsResponseOk {
        vehicles: vehicles.into_iter().filter_map(Vehicle::from_otp).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::gtfs_graphql::VehiclePosition;

    fn positioned(lat: Option<f64>, lon: Option<f64>) -> gtfs_graphql::PatternVehicle {
        gtfs_graphql::PatternVehicle {
            pattern_code: "1:40:0:01".to_owned(),
            position: VehiclePosition {
                vehicle_id: Some("1:7204".to_owned()),
                label: Some("7204".to_owned()),
                lat,
                lon,
                heading: Some(180.0),
                last_update: None,
            },
        }
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
        assert!(Vehicle::from_otp(positioned(Some(47.6), Some(-122.33))).is_some());
        assert!(Vehicle::from_otp(positioned(Some(47.6), None)).is_none());
        assert!(Vehicle::from_otp(positioned(None, None)).is_none());
    }
}
