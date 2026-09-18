//! Where the vehicles serving a plan's transit legs are right now.
//!
//! Clients poll this against the `patternCode` of the transit legs in a plan they're already
//! showing. Pattern codes are only meaningful to the OTP instance that issued them, so the trip's
//! endpoints come along to pick the same router the plan came from.

use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use geo::geometry::{LineString, Point};
use polyline::decode_polyline;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::error::PlanResponseErr;
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql;
use crate::util::serde_util::deserialize_point_from_lat_lon;
use crate::util::{point_at, progress_along};
use crate::Error;

/// OTP encodes its polylines at 1e-5, the original Google scale.
const OTP_POLYLINE_PRECISION: u32 = 5;

/// How far ahead of a vehicle's last report we're willing to guess. Beyond this the predictions
/// it's built from are worth less than admitting we don't know.
const TRACK_HORIZON: TimeDelta = TimeDelta::minutes(3);

/// How many vehicles still short of the boarding stop to report. These are the ones that might
/// actually pick the rider up.
const UPCOMING_VEHICLES: usize = 2;

/// How many vehicles past the boarding stop to report. One is enough to show the rider what they
/// just missed; more is clutter from a bus that's no longer theirs.
const DEPARTED_VEHICLES: usize = 1;

/// How finely the guess is sampled. The client walks between samples in a straight line, so this
/// only has to be short enough that a bus doesn't round a corner inside one.
const TRACK_STEP: TimeDelta = TimeDelta::seconds(5);

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePositionsQuery {
    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    from_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    to_place: Point,

    /// The patterns to report on, `;` separated, as `<code>` or `<code>@<lat>,<lon>`.
    ///
    /// The code is a plan's transit leg's `patternCode`. The optional point is where the rider
    /// boards that leg, which is what "nearby" is measured from - a vehicle two miles up the
    /// route is on the same pattern but is nothing to do with the trip.
    patterns: String,
}

/// One pattern a client asked about.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternRequest {
    code: String,
    /// Where the rider boards. Without it every vehicle on the pattern comes back, since there's
    /// nothing to rank them against.
    boarding_stop: Option<Point>,
}

impl VehiclePositionsQuery {
    fn patterns(&self) -> Vec<PatternRequest> {
        self.patterns
            .split(';')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| match entry.split_once('@') {
                Some((code, point)) => PatternRequest {
                    code: code.to_owned(),
                    boarding_stop: parse_lat_lon(point),
                },
                None => PatternRequest {
                    code: entry.to_owned(),
                    boarding_stop: None,
                },
            })
            .collect()
    }
}

/// `<lat>,<lon>`, the way every other point in this API is written.
fn parse_lat_lon(point: &str) -> Option<Point> {
    let (lat, lon) = point.split_once(',')?;
    Some(Point::new(
        lon.trim().parse().ok()?,
        lat.trim().parse().ok()?,
    ))
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

    /// Degrees clockwise from north, as the feed reported it. Most feeds don't publish one.
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<f64>,

    /// RFC 3339. When the vehicle reported this position.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_updated: Option<DateTime<FixedOffset>>,

    /// How far along the pattern's shape this vehicle is, in metres. Kept for ranking vehicles
    /// against the boarding stop, which is a server-side concern.
    #[serde(skip)]
    progress: Option<f64>,

    /// Where we guess the vehicle goes next, for a client to animate along between polls.
    ///
    /// Sampled every few seconds from the last report up to at most a few minutes out, by
    /// walking the route's shape at the pace the trip's arrival predictions imply. Every point
    /// past the first is a guess, not a report - `last_updated` is still the last thing the
    /// vehicle actually told us. Empty when there's nothing to predict from, in which case the
    /// vehicle should just sit at `lat`/`lon`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    track: Vec<Waypoint>,
}

/// Where a vehicle is guessed to be at one moment.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Waypoint {
    lat: f64,
    lon: f64,
    /// RFC 3339, UTC.
    time: DateTime<Utc>,
}

impl Waypoint {
    /// Interpolating a shape yields far more digits than it means. A millionth of a degree is
    /// about 10cm, already finer than the GPS fix underneath, and the rest is payload.
    fn new(point: Point, time: DateTime<Utc>) -> Self {
        let round = |degrees: f64| (degrees * 1e6).round() / 1e6;
        Self {
            lat: round(point.y()),
            lon: round(point.x()),
            time,
        }
    }
}

/// A point the vehicle is expected to reach, and when - the vehicle's own position to begin
/// with, then each upcoming stop at its predicted arrival.
struct Anchor {
    progress: f64,
    time: DateTime<Utc>,
}

/// The stops ahead of `position` that the trip still expects to reach, in order.
///
/// Predictions that don't move both forward along the shape and forward in time are dropped:
/// OTP hands back the whole day's stop sequence, including stops the vehicle has already passed
/// and, on a loop, stops whose shape position is behind it.
fn anchors(
    shape: &LineString,
    progress: f64,
    reported_at: DateTime<Utc>,
    stoptimes: &[gtfs_graphql::VehicleStoptime],
) -> Vec<Anchor> {
    let mut anchors = vec![Anchor {
        progress,
        time: reported_at,
    }];
    let horizon = reported_at + TRACK_HORIZON;

    for stoptime in stoptimes {
        let (Some(point), Some(time)) = (stoptime.point(), stoptime.expected_arrival()) else {
            continue;
        };
        let Some(stop) = progress_along(shape, point) else {
            continue;
        };
        let last = anchors.last().expect("seeded above");
        if stop <= last.progress || time <= last.time {
            continue;
        }
        anchors.push(Anchor {
            progress: stop,
            time,
        });
        if time >= horizon {
            break;
        }
    }
    anchors
}

/// Sample the guessed path at a fixed cadence, so a client can walk it by wall clock.
fn track(shape: &LineString, anchors: &[Anchor]) -> Vec<Waypoint> {
    let (Some(first), Some(last)) = (anchors.first(), anchors.last()) else {
        return vec![];
    };
    // One anchor is just the vehicle where it already is - nothing to say.
    if anchors.len() < 2 {
        return vec![];
    }

    let mut waypoints = Vec::new();
    let mut time = first.time;
    let end = last.time.min(first.time + TRACK_HORIZON);
    while time <= end {
        // The pair of anchors this instant falls between.
        let next = anchors
            .iter()
            .position(|anchor| anchor.time > time)
            .unwrap_or(anchors.len() - 1);
        let before = &anchors[next.saturating_sub(1)];
        let after = &anchors[next];

        let span = (after.time - before.time).num_milliseconds() as f64;
        let into = if span > 0.0 {
            (time - before.time).num_milliseconds() as f64 / span
        } else {
            0.0
        };
        let progress = before.progress + into * (after.progress - before.progress);

        if let Some(point) = point_at(shape, progress) {
            waypoints.push(Waypoint::new(point, time));
        }
        time += TRACK_STEP;
    }
    waypoints
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
        let progress = shape.and_then(|shape| progress_along(shape, Point::new(lon, lat)));

        // Guessing forward only makes sense from a report we can date.
        let track = match (shape, progress, position.last_update) {
            (Some(shape), Some(progress), Some(reported_at)) => {
                let stoptimes: Vec<_> = position
                    .trip
                    .stoptimes_for_date
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .collect();
                let anchors = anchors(shape, progress, reported_at.with_timezone(&Utc), &stoptimes);
                track(shape, &anchors)
            }
            _ => vec![],
        };

        Some(Self {
            pattern_code: pattern_code.to_owned(),
            vehicle_id: position.vehicle_id,
            label: position.label,
            lat,
            lon,
            heading: position.heading,
            progress,
            last_updated: position.last_update,
            track,
        })
    }
}

/// The handful of vehicles worth drawing for a rider boarding at `boarding_stop`: the ones just
/// short of it, and the one that just left.
///
/// A pattern runs its whole length, so most of its vehicles are miles from the trip and only
/// crowd the map. Returns everything, unranked, when there's nothing to rank against - no
/// boarding stop, or a pattern whose shape wouldn't decode.
fn nearby(
    mut vehicles: Vec<Vehicle>,
    shape: Option<&LineString>,
    boarding_stop: Option<Point>,
) -> Vec<Vehicle> {
    let Some(boarding) = shape
        .zip(boarding_stop)
        .and_then(|(shape, stop)| progress_along(shape, stop))
    else {
        return vehicles;
    };

    // Nearest first on each side of the stop. A vehicle we couldn't place on the shape sorts to
    // the back rather than being dropped - it's still really out there.
    vehicles.sort_by(|a, b| {
        let key = |vehicle: &Vehicle| {
            vehicle
                .progress
                .map(|progress| (progress - boarding).abs())
                .unwrap_or(f64::MAX)
        };
        key(a).partial_cmp(&key(b)).unwrap_or(Ordering::Equal)
    });

    let mut upcoming = 0;
    let mut departed = 0;
    vehicles.retain(|vehicle| {
        let Some(progress) = vehicle.progress else {
            return false;
        };
        // Sitting exactly at the stop counts as still to come: it hasn't left yet.
        if progress <= boarding {
            upcoming += 1;
            upcoming <= UPCOMING_VEHICLES
        } else {
            departed += 1;
            departed <= DEPARTED_VEHICLES
        }
    });
    vehicles
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
    let requested = query.patterns();
    if requested.is_empty() {
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
    let codes = requested.iter().map(|p| p.code.clone()).collect();
    let vehicles = gtfs_graphql::vehicle_positions(&client, &endpoint, codes)
        .await
        .map_err(|e| {
            log::error!("error while fetching vehicle positions from otp service: {e}");
            PlanResponseErr::from(e)
        })?;

    let vehicles = vehicles
        .into_iter()
        .flat_map(|pattern| {
            let shape = shape_of(&pattern);
            let boarding_stop = requested
                .iter()
                .find(|request| request.code == pattern.pattern_code)
                .and_then(|request| request.boarding_stop);
            let on_pattern: Vec<_> = pattern
                .positions
                .into_iter()
                .filter_map(|position| {
                    Vehicle::from_otp(&pattern.pattern_code, shape.as_ref(), position)
                })
                .collect();
            nearby(on_pattern, shape.as_ref(), boarding_stop)
        })
        .collect();

    Ok(VehiclePositionsResponseOk { vehicles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::gtfs_graphql::{StoptimeStop, VehiclePosition, VehicleStoptime, VehicleTrip};
    use geo::line_string;

    /// Midnight of the service day these fixtures run on.
    const SERVICE_DAY: i64 = 1_716_015_600;

    fn at(after_midnight: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(SERVICE_DAY + after_midnight, 0).expect("in range")
    }

    /// A straight run due east. A degree of longitude here is about 75km, so the stops below sit
    /// roughly 750m apart.
    fn shape() -> LineString {
        line_string![
            (x: -122.340, y: 47.600),
            (x: -122.300, y: 47.600),
        ]
    }

    fn stoptime(lon: f64, arrival: i32) -> VehicleStoptime {
        VehicleStoptime {
            realtime: Some(true),
            realtime_arrival: Some(arrival),
            scheduled_arrival: Some(arrival),
            service_day: Some(SERVICE_DAY),
            stop: Some(StoptimeStop {
                lat: Some(47.600),
                lon: Some(lon),
            }),
        }
    }

    fn position(lat: Option<f64>, lon: Option<f64>) -> VehiclePosition {
        VehiclePosition {
            vehicle_id: Some("1:7204".to_owned()),
            label: Some("7204".to_owned()),
            lat,
            lon,
            heading: None,
            last_update: None,
            trip: VehicleTrip {
                gtfs_id: "1:809330321".to_owned(),
                stoptimes_for_date: None,
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

    fn codes_of(patterns: &[PatternRequest]) -> Vec<&str> {
        patterns.iter().map(|p| p.code.as_str()).collect()
    }

    #[test]
    fn splits_patterns() {
        let patterns = query("1:40:0:01;1:21:0:01").patterns();
        assert_eq!(codes_of(&patterns), ["1:40:0:01", "1:21:0:01"]);
        assert!(patterns.iter().all(|p| p.boarding_stop.is_none()));

        assert!(query("").patterns().is_empty());
        // A plan whose transit legs all lack a pattern sends an empty element rather than nothing.
        assert_eq!(codes_of(&query("1:40:0:01;").patterns()), ["1:40:0:01"]);
    }

    #[test]
    fn reads_the_boarding_stop_off_a_pattern() {
        let patterns = query("1:40:0:01@47.6,-122.33;1:21:0:01").patterns();
        assert_eq!(codes_of(&patterns), ["1:40:0:01", "1:21:0:01"]);
        assert_eq!(patterns[0].boarding_stop, Some(Point::new(-122.33, 47.6)));
        // Not every leg has to name one.
        assert_eq!(patterns[1].boarding_stop, None);
    }

    /// Better to report on the whole pattern than to drop it over a malformed point.
    #[test]
    fn an_unreadable_boarding_stop_leaves_the_pattern_unranked() {
        let patterns = query("1:40:0:01@not-a-point").patterns();
        assert_eq!(codes_of(&patterns), ["1:40:0:01"]);
        assert_eq!(patterns[0].boarding_stop, None);
    }

    /// Vehicles strung along the shape, identified by their longitude.
    fn vehicles_at(lons: &[f64]) -> Vec<Vehicle> {
        let shape = shape();
        lons.iter()
            .map(|lon| {
                let mut position = position(Some(47.600), Some(*lon));
                position.label = Some(format!("{lon}"));
                Vehicle::from_otp("1:40:0:01", Some(&shape), position).expect("has coordinates")
            })
            .collect()
    }

    fn labels(vehicles: &[Vehicle]) -> Vec<&str> {
        vehicles
            .iter()
            .map(|v| v.label.as_deref().unwrap_or(""))
            .collect()
    }

    #[test]
    fn keeps_the_two_vehicles_approaching_the_stop_and_the_one_that_just_left() {
        let shape = shape();
        // The shape runs east, so a smaller longitude is further back along it. Spaced so no
        // two are equidistant from the stop, which would leave the order up to the sort.
        let vehicles = vehicles_at(&[-122.339, -122.336, -122.332, -122.329, -122.325, -122.305]);
        let boarding_stop = Point::new(-122.330, 47.600);

        let nearby = nearby(vehicles, Some(&shape), Some(boarding_stop));

        // Nearest first: the one just past the stop, then the two still approaching it.
        // -122.339 is a third vehicle still approaching, -122.325 and -122.305 are further past.
        assert_eq!(labels(&nearby), ["-122.329", "-122.332", "-122.336"]);
    }

    #[test]
    fn a_pattern_with_no_boarding_stop_reports_every_vehicle() {
        let shape = shape();
        let vehicles = vehicles_at(&[-122.339, -122.335, -122.331, -122.329, -122.305]);

        assert_eq!(nearby(vehicles.clone(), Some(&shape), None).len(), 5);
        // Nor can we rank without a shape to measure along.
        assert_eq!(
            nearby(vehicles, None, Some(Point::new(-122.330, 47.600))).len(),
            5
        );
    }

    #[test]
    fn fewer_vehicles_than_we_would_show_is_fine() {
        let shape = shape();
        let vehicles = vehicles_at(&[-122.335]);
        let nearby = nearby(vehicles, Some(&shape), Some(Point::new(-122.330, 47.600)));

        assert_eq!(labels(&nearby), ["-122.335"]);
    }

    #[test]
    fn a_vehicle_without_coordinates_is_dropped() {
        let shape = shape();
        let build = |lat, lon| Vehicle::from_otp("1:40:0:01", Some(&shape), position(lat, lon));
        assert!(build(Some(47.6), Some(-122.33)).is_some());
        assert!(build(Some(47.6), None).is_none());
        assert!(build(None, None).is_none());
    }

    /// OTP has no shape for some patterns. Their vehicles are still worth drawing.
    #[test]
    fn a_pattern_without_a_shape_still_yields_a_vehicle() {
        let vehicle = Vehicle::from_otp("1:40:0:01", None, position(Some(47.6), Some(-122.335)))
            .expect("has coordinates");
        assert!(vehicle.progress.is_none());
        assert!(vehicle.track.is_empty());
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

    fn anchors_for(
        vehicle_lon: f64,
        reported_at: i64,
        stoptimes: &[VehicleStoptime],
    ) -> Vec<Anchor> {
        let shape = shape();
        let progress =
            progress_along(&shape, Point::new(vehicle_lon, 47.600)).expect("on the shape");
        anchors(&shape, progress, at(reported_at), stoptimes)
    }

    /// OTP hands back the whole day's stop sequence, most of which is behind the vehicle.
    #[test]
    fn stops_the_vehicle_has_already_passed_are_dropped() {
        let stoptimes = [
            stoptime(-122.340, 100), // start of the line, well behind
            stoptime(-122.330, 200), // behind
            stoptime(-122.320, 400), // ahead
            stoptime(-122.310, 600), // ahead
        ];
        let anchors = anchors_for(-122.325, 300, &stoptimes);

        // The vehicle itself, then only the two stops ahead of it.
        assert_eq!(anchors.len(), 3);
        assert!(anchors[1].progress > anchors[0].progress);
        assert_eq!(anchors[1].time, at(400));
        assert_eq!(anchors[2].time, at(600));
    }

    /// The position and the predictions come from the same minute-old snapshot, so by the time we
    /// serve them the next stop's arrival can already be in the past. Those can't anchor
    /// anything - the walk has to carry on to a prediction that's still ahead.
    #[test]
    fn predictions_that_have_already_expired_are_walked_past() {
        let stoptimes = [
            stoptime(-122.320, 250), // ahead on the shape, but due before the report
            stoptime(-122.310, 600),
        ];
        let anchors = anchors_for(-122.325, 300, &stoptimes);

        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[1].time, at(600));
    }

    #[test]
    fn a_vehicle_with_nothing_ahead_of_it_gets_no_track() {
        let stoptimes = [stoptime(-122.340, 100), stoptime(-122.330, 200)];
        let anchors = anchors_for(-122.325, 300, &stoptimes);

        assert_eq!(anchors.len(), 1);
        assert!(track(&shape(), &anchors).is_empty());
    }

    #[test]
    fn the_track_starts_where_the_vehicle_is_and_walks_toward_the_next_stop() {
        let stoptimes = [stoptime(-122.320, 400), stoptime(-122.310, 600)];
        let anchors = anchors_for(-122.325, 300, &stoptimes);
        let track = track(&shape(), &anchors);

        assert!(
            track.len() > 2,
            "expected several samples, got {}",
            track.len()
        );
        assert_eq!(track[0].time, at(300));
        // Sampled at a fixed cadence...
        assert_eq!(track[1].time - track[0].time, TRACK_STEP);
        // ...running east, the way the shape does, and never past the last prediction.
        assert!(track[1].lon > track[0].lon);
        assert!(track.last().expect("non-empty").time <= at(600));
        assert!(track.windows(2).all(|pair| pair[1].lon >= pair[0].lon));
    }

    /// A trip predicted hours out shouldn't produce hours of guessed positions.
    #[test]
    fn the_track_stops_at_the_horizon() {
        let stoptimes = [stoptime(-122.310, 300 + 60 * 60)];
        let anchors = anchors_for(-122.325, 300, &stoptimes);
        let track = track(&shape(), &anchors);

        let span = track.last().expect("non-empty").time - track[0].time;
        assert!(span <= TRACK_HORIZON, "track ran {span} past the horizon");
    }
}
