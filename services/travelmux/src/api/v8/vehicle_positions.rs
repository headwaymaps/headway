//! Where the vehicles serving a plan's transit legs are right now.
//!
//! Clients poll this against the `patternCode` of the transit legs in a plan they're already
//! showing. Pattern codes are only meaningful to the OTP instance that issued them, so the trip's
//! endpoints come along to pick the same router the plan came from.

use actix_web::{post, web, HttpRequest, HttpResponseBuilder, Responder};
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use geo::geometry::{LineString, Point};
use geo::{Haversine, InterpolateLine};
use polyline::decode_polyline;
use serde::{Deserialize, Serialize};

use super::error::PlanResponseErr;
use super::plan::Route;
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql;
use crate::util::progress_along;
use crate::util::serde_util::{
    deserialize_optional_point_from_lon_lat_pair, deserialize_point_from_lon_lat_pair,
    serialize_point_as_lon_lat_pair, serialize_points_as_lon_lat_pairs,
};
use crate::Error;

/// OTP encodes its polylines at 1e-5, the original Google scale.
const OTP_POLYLINE_PRECISION: u32 = 5;

/// How far ahead of a vehicle's last report we're willing to guess. Beyond this the predictions
/// it's built from are worth less than admitting we don't know.
const TRACK_HORIZON: TimeDelta = TimeDelta::minutes(3);

/// The fastest a transit vehicle plausibly averages between two stops, in metres per second.
///
/// 30 m/s is 108 km/h. Measured against a day of Puget Sound vehicles, the implied speed to a
/// vehicle's own next stop sits at 4.5 m/s in the median and 22.3 at the 99th percentile, so
/// this only ever catches data contradicting itself - a Sounder run between its widest-spaced
/// stops works out around 19 m/s.
const MAX_PLAUSIBLE_SPEED: f64 = 30.0;

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
pub struct VehiclePositionsRequest {
    #[serde(deserialize_with = "deserialize_point_from_lon_lat_pair")]
    from_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lon_lat_pair")]
    to_place: Point,

    /// The patterns to report on. A code is a plan's transit leg's `patternCode`.
    patterns: Vec<PatternRequest>,
}

/// One pattern a client asked about.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatternRequest {
    code: String,

    /// Where the rider boards, which is what "nearby" is measured from - a vehicle two miles up
    /// the route is on the same pattern but is nothing to do with the trip. Without it every
    /// vehicle on the pattern comes back, since there's nothing to rank them against.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_point_from_lon_lat_pair"
    )]
    boarding_stop: Option<Point>,
}

/// Whether a coordinate is somewhere a vehicle could actually be.
///
/// Null island is the interesting case: OTP reports a vehicle at exactly `0, 0` for a run that
/// hasn't started, which is a thousand miles off the coast of Ghana rather than a position.
fn is_on_earth(point: Point) -> bool {
    let (lon, lat) = (point.x(), point.y());
    lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
        && (lat != 0.0 || lon != 0.0)
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePositionsResponseOk {
    /// Requested patterns the graph has never heard of, which is how a client tells "this route
    /// isn't running right now" from "these codes died when the transit data was rebuilt". Both
    /// otherwise look like an empty vehicle list, and only the second means re-plan.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unknown_patterns: Vec<String>,

    /// RFC 3339, UTC. What the clock said here as this was answered.
    ///
    /// Every time in this response is the server's, and a client animating against its own clock
    /// is animating against a different one. Comparing this to the moment the response arrived
    /// gives the offset between them, which is what a track should be walked by - a device a few
    /// minutes out would otherwise hold every vehicle at one end of its track or the other.
    server_time: DateTime<Utc>,
    vehicles: Vec<Vehicle>,
}

/// One transit vehicle's last known position.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    /// Stable for as long as the vehicle keeps reporting on this pattern, so a client can match
    /// it against what it drew last poll rather than inventing a key of its own.
    id: String,

    /// Which of the requested patterns this vehicle is serving.
    pattern_code: String,

    /// What the vehicle is running. Repeated on every vehicle rather than left for the client to
    /// look up from the plan's legs, so each client doesn't write that join again.
    route: Route,

    /// What kind of vehicle it is, e.g. `BUS` or `TRAM`, as OTP names it.
    #[serde(skip_serializing_if = "Option::is_none")]
    vehicle_mode: Option<gtfs_graphql::TransitMode>,

    /// What the vehicle displays, e.g. "Downtown Seattle Via 35th Ave SW".
    #[serde(skip_serializing_if = "Option::is_none")]
    headsign: Option<String>,

    /// `FeedId:VehicleId`, unique for as long as the vehicle is reporting.
    #[serde(skip_serializing_if = "Option::is_none")]
    vehicle_id: Option<String>,

    /// What the vehicle shows the public, e.g. a fleet number.
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    /// Where the vehicle last reported being, as `[lon, lat]`.
    #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
    position: Point,

    /// Degrees clockwise from north, as the feed reported it. Most feeds don't publish one.
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<f64>,

    /// RFC 3339. When the vehicle reported this position.
    last_updated: DateTime<FixedOffset>,

    /// How far along the pattern's shape this vehicle is, in metres. Kept for ranking vehicles
    /// against the boarding stop, which is a server-side concern.
    #[serde(skip)]
    progress: f64,

    /// Where we guess the vehicle goes next, for a client to animate along between polls.
    ///
    /// Absent when there's nothing to predict from, in which case the vehicle just sits at
    /// `position`.
    #[serde(skip_serializing_if = "Option::is_none")]
    track: Option<Track>,
}

/// Where a vehicle is expected to be over the next few minutes, sampled at a fixed cadence by
/// walking the route's shape at the pace the trip's arrival predictions imply.
///
/// The track begins at the vehicle's own `lastUpdated`: `points[0]` is where it was when it
/// reported, and everything after is a guess. The samples are evenly spaced in time, so the pair
/// bracketing any instant is arithmetic rather than a search:
/// `i = (now - lastUpdated) / stepSeconds`, clamped to `points`.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Seconds between consecutive points.
    step_seconds: i64,
    /// At least two points, serialized as `[lon, lat]` pairs. The first is `lastUpdated`'s.
    #[serde(serialize_with = "serialize_points_as_lon_lat_pairs")]
    points: Vec<Point>,
}

/// Interpolating a shape yields far more digits than it means. A millionth of a degree is about
/// 10cm, already finer than the GPS fix underneath, and the rest is payload.
fn rounded(point: Point) -> Point {
    let round = |degrees: f64| (degrees * 1e6).round() / 1e6;
    Point::new(round(point.x()), round(point.y()))
}

/// A point the vehicle is expected to reach, and when - the vehicle's own position to begin
/// with, then each upcoming stop at its predicted arrival.
struct Anchor {
    progress: f64,
    time: DateTime<Utc>,
}

/// The stops the trip still expects to reach, in order, starting from the one OTP says the
/// vehicle is working towards.
///
/// Starting there rather than at "the first prediction still in the future" is what keeps a
/// vehicle on the ground. OTP hands back the whole day's stop sequence, and when a run is late
/// its next several predictions are already in the past; scanning past them lands on a stop
/// kilometres away with seconds left to reach it, and the vehicle is flung down the route. The
/// stop OTP names is the one that agrees with the position it reported alongside.
///
/// Returns just the vehicle's own position - and so no track at all - when there's no stop to
/// aim at, or when the stop it names is already behind it or overdue. Sitting still is honest;
/// guessing is what put a bus across town.
fn anchors(
    shape: &LineString,
    progress: f64,
    reported_at: DateTime<Utc>,
    heading_for: Option<&str>,
    stoptimes: &[gtfs_graphql::VehicleStoptime],
) -> Vec<Anchor> {
    let mut anchors = vec![Anchor {
        progress,
        time: reported_at,
    }];
    let horizon = reported_at + TRACK_HORIZON;

    let Some(from) =
        heading_for.and_then(|next| stoptimes.iter().position(|s| s.stop_id() == Some(next)))
    else {
        return anchors;
    };

    for stoptime in &stoptimes[from..] {
        let (Some(point), Some(time)) = (stoptime.point(), stoptime.expected_arrival()) else {
            continue;
        };
        let Some(stop) = progress_along(shape, point) else {
            continue;
        };
        let last = anchors.last().expect("seeded above");
        // An overdue next stop makes the report and prediction inconsistent.
        if time <= last.time {
            if anchors.len() == 1 {
                return anchors;
            }
            continue;
        }
        // At or past the next stop means it has arrived; aim farther ahead.
        if stop <= last.progress {
            continue;
        }
        // Stop when the report and predictions imply an implausible speed.
        let seconds = (time - last.time).num_milliseconds() as f64 / 1000.0;
        if (stop - last.progress) / seconds > MAX_PLAUSIBLE_SPEED {
            break;
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
fn track(shape: &LineString, anchors: &[Anchor]) -> Option<Track> {
    // Fewer than two anchors is just the vehicle where it already is - nothing to say.
    let [first, .., last] = anchors else {
        return None;
    };

    let mut points = Vec::new();
    let mut time = first.time;
    let end = last.time.min(first.time + TRACK_HORIZON);
    while time <= end {
        // The pair of anchors this instant falls between. Never the first anchor: `time` starts
        // at its own time and the search is for one strictly later.
        let next = anchors
            .iter()
            .position(|anchor| anchor.time > time)
            .unwrap_or(anchors.len() - 1);
        let before = &anchors[next - 1];
        let after = &anchors[next];

        // Anchors are strictly increasing in time, so the span is never zero.
        let span = (after.time - before.time).num_milliseconds() as f64;
        let into = (time - before.time).num_milliseconds() as f64 / span;
        let progress = before.progress + into * (after.progress - before.progress);

        if let Some(point) = Haversine.point_at_distance_from_start(shape, progress) {
            points.push(rounded(point));
        }
        time += TRACK_STEP;
    }

    // A single point is a position, not a path; the vehicle already has one of those.
    (points.len() > 1).then(|| Track {
        step_seconds: TRACK_STEP.num_seconds(),
        points,
    })
}

impl Vehicle {
    /// A vehicle we can't place on the shape at a known moment has nothing to draw, so it's
    /// dropped rather than represented. OTP reports a vehicle at null island for a run it hasn't
    /// started yet, twice over: once properly, and once as a placeholder sharing the real one's
    /// id.
    fn from_otp(
        pattern: &gtfs_graphql::PatternVehicles,
        shape: &LineString,
        position: gtfs_graphql::VehiclePosition,
    ) -> Option<Self> {
        let pattern_code = pattern.pattern_code.as_str();
        let reported_at_point = Point::new(position.lon?, position.lat?);
        if !is_on_earth(reported_at_point) {
            return None;
        }
        let progress = progress_along(shape, reported_at_point)?;
        // A report we can't date can't be guessed forward from, and can't be honestly aged for
        // the rider either.
        let last_updated = position.last_update?;

        let heading_for = position
            .stop_relationship
            .as_ref()
            .map(|relationship| relationship.stop.gtfs_id.clone());

        let stoptimes: Vec<_> = position
            .trip
            .stoptimes_for_date
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();
        let anchors = anchors(
            shape,
            progress,
            last_updated.with_timezone(&Utc),
            heading_for.as_deref(),
            &stoptimes,
        );
        let track = track(shape, &anchors);

        // A vehicle reports under one id for as long as it's on this pattern, so that plus the
        // pattern is stable between polls - which is all a client needs to keep a marker.
        let identifier = position
            .vehicle_id
            .as_deref()
            .or(position.label.as_deref())
            .unwrap_or_default();

        Some(Self {
            id: format!("{pattern_code}/{identifier}"),
            pattern_code: pattern_code.to_owned(),
            route: Route {
                short_name: pattern.route.short_name.clone(),
                long_name: pattern.route.long_name.clone(),
                color: pattern.route.color.clone(),
            },
            vehicle_mode: pattern.route.mode.clone(),
            headsign: pattern.headsign.clone(),
            vehicle_id: position.vehicle_id,
            label: position.label,
            position: reported_at_point,
            heading: position.heading,
            progress,
            last_updated,
            track,
        })
    }
}

/// The handful of vehicles worth drawing for a rider boarding at `boarding_stop`: the ones just
/// short of it, and the one that just left.
///
/// A pattern runs its whole length, so most of its vehicles are miles from the trip and only
/// crowd the map. Returns everything, unranked, when there's no boarding stop to rank against.
fn nearby(
    mut vehicles: Vec<Vehicle>,
    shape: &LineString,
    boarding_stop: Option<Point>,
) -> Vec<Vehicle> {
    let Some(boarding) = boarding_stop.and_then(|stop| progress_along(shape, stop)) else {
        return vehicles;
    };

    // Nearest first, on whichever side of the stop it's on.
    vehicles.sort_by(|a, b| {
        let key = |vehicle: &Vehicle| (vehicle.progress - boarding).abs();
        key(a).total_cmp(&key(b))
    });

    let mut upcoming = 0;
    let mut departed = 0;
    vehicles.retain(|vehicle| {
        if vehicle.progress <= boarding {
            upcoming += 1;
            upcoming <= UPCOMING_VEHICLES
        } else {
            departed += 1;
            departed <= DEPARTED_VEHICLES
        }
    });
    vehicles
}

/// Decodes a pattern shape when OTP provides one.
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

#[post("/v8/vehicle_positions")]
pub async fn post_vehicle_positions(
    request: web::Json<VehiclePositionsRequest>,
    app_state: web::Data<AppState>,
) -> std::result::Result<VehiclePositionsResponseOk, PlanResponseErr> {
    let requested = &request.patterns;
    if requested.is_empty() {
        return Ok(VehiclePositionsResponseOk {
            unknown_patterns: vec![],
            server_time: Utc::now(),
            vehicles: vec![],
        });
    }

    let endpoint = {
        let Some(router) = app_state
            .otp_cluster()
            .find_router(request.from_place, request.to_place)
        else {
            Err(
                Error::user("Transit directions not available for this area.")
                    .error_type(ErrorType::NoCoverageForArea),
            )?
        };
        router.endpoint().clone()
    };

    let client = reqwest::Client::new();
    // A code repeated in the query would have OTP return its vehicles once per mention, and
    // each copy would be drawn as though it were a separate bus.
    let mut codes: Vec<String> = requested.iter().map(|p| p.code.clone()).collect();
    codes.sort();
    codes.dedup();
    let vehicles = gtfs_graphql::vehicle_positions(&client, &endpoint, codes)
        .await
        .map_err(|e| {
            log::error!("error while fetching vehicle positions from otp service: {e}");
            PlanResponseErr::from(e)
        })?;

    let known: std::collections::HashSet<String> = vehicles
        .iter()
        .map(|pattern| pattern.pattern_code.clone())
        .collect();

    let vehicles = vehicles
        .into_iter()
        .flat_map(|pattern| {
            let Some(shape) = shape_of(&pattern) else {
                return Vec::new();
            };
            let boarding_stop = requested
                .iter()
                .find(|request| request.code == pattern.pattern_code)
                .and_then(|request| request.boarding_stop);
            let mut pattern = pattern;
            let positions = std::mem::take(&mut pattern.positions);
            let on_pattern: Vec<_> = positions
                .into_iter()
                .filter_map(|position| Vehicle::from_otp(&pattern, &shape, position))
                .collect();
            nearby(on_pattern, &shape, boarding_stop)
        })
        .collect();

    Ok(VehiclePositionsResponseOk {
        unknown_patterns: requested
            .iter()
            .map(|request| request.code.clone())
            .filter(|code| !known.contains(code))
            .collect(),
        server_time: Utc::now(),
        vehicles,
    })
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

    /// A stop at `lon` along the shape, due `arrival` seconds after midnight. Its id names its
    /// longitude, so a test can say which stop the vehicle is heading for.
    fn stoptime(lon: f64, arrival: i32) -> VehicleStoptime {
        VehicleStoptime {
            realtime: Some(true),
            realtime_arrival: Some(arrival),
            scheduled_arrival: Some(arrival),
            service_day: Some(SERVICE_DAY),
            stop: Some(StoptimeStop {
                gtfs_id: format!("stop@{lon}"),
                lat: Some(47.600),
                lon: Some(lon),
            }),
        }
    }

    fn stop_id(lon: f64) -> String {
        format!("stop@{lon}")
    }

    fn pattern() -> gtfs_graphql::PatternVehicles {
        gtfs_graphql::PatternVehicles {
            pattern_code: "1:40:0:01".to_owned(),
            geometry: None,
            route: gtfs_graphql::VehicleRoute {
                short_name: Some("40".to_owned()),
                long_name: Some("Downtown - Ballard".to_owned()),
                color: Some("0080FF".to_owned()),
                mode: Some(gtfs_graphql::TransitMode::Bus),
            },
            headsign: Some("Downtown Seattle".to_owned()),
            positions: vec![],
        }
    }

    fn position(lat: Option<f64>, lon: Option<f64>) -> VehiclePosition {
        VehiclePosition {
            vehicle_id: Some("1:7204".to_owned()),
            label: Some("7204".to_owned()),
            lat,
            lon,
            heading: None,
            last_update: Some(at(300).fixed_offset()),
            stop_relationship: None,
            trip: VehicleTrip {
                gtfs_id: "1:809330321".to_owned(),
                stoptimes_for_date: None,
            },
        }
    }

    fn request_for(patterns: serde_json::Value) -> VehiclePositionsRequest {
        serde_json::from_value(serde_json::json!({
            "fromPlace": [-122.34, 47.57],
            "toPlace": [-122.34, 47.65],
            "patterns": patterns,
        }))
        .expect("deserializes")
    }

    fn codes_of(patterns: &[PatternRequest]) -> Vec<&str> {
        patterns.iter().map(|p| p.code.as_str()).collect()
    }

    #[test]
    fn reads_the_patterns_asked_about() {
        let request = request_for(serde_json::json!([
            { "code": "1:40:0:01" },
            { "code": "1:21:0:01" },
        ]));
        assert_eq!(codes_of(&request.patterns), ["1:40:0:01", "1:21:0:01"]);
        assert!(request.patterns.iter().all(|p| p.boarding_stop.is_none()));

        assert!(request_for(serde_json::json!([])).patterns.is_empty());
    }

    #[test]
    fn reads_the_boarding_stop_off_a_pattern() {
        let request = request_for(serde_json::json!([
            { "code": "1:40:0:01", "boardingStop": [-122.33, 47.6] },
            { "code": "1:21:0:01" },
        ]));
        assert_eq!(
            request.patterns[0].boarding_stop,
            Some(Point::new(-122.33, 47.6))
        );
        // Not every leg has to name one.
        assert_eq!(request.patterns[1].boarding_stop, None);
    }

    /// Boarding stops are parsed as the same `[lon, lat]` pair as every other v8 input.
    #[test]
    fn reads_a_boarding_stop_without_position_validation() {
        for off_globe in [[-122.33, 91.0], [181.0, 47.6], [0.0, 0.0]] {
            let request = request_for(serde_json::json!([
                { "code": "1:40:0:01", "boardingStop": off_globe },
            ]));
            assert_eq!(codes_of(&request.patterns), ["1:40:0:01"], "{off_globe:?}");
            assert_eq!(
                request.patterns[0].boarding_stop,
                Some(Point::from(off_globe)),
                "{off_globe:?}"
            );
        }
    }

    /// A pair, not the `lat`/`lon` object v7 used.
    #[test]
    fn a_point_is_read_as_lon_lat() {
        let request = request_for(serde_json::json!([]));
        assert_eq!(request.from_place, Point::new(-122.34, 47.57));
        assert!(
            serde_json::from_value::<VehiclePositionsRequest>(serde_json::json!({
                "fromPlace": { "lat": 47.57, "lon": -122.34 },
                "toPlace": [-122.34, 47.65],
                "patterns": [],
            }))
            .is_err()
        );
    }

    /// The client keys its markers on this, so it has to survive a poll and tell two vehicles on
    /// the same pattern apart.
    #[test]
    fn a_vehicle_is_identified_by_its_pattern_and_its_own_id() {
        let shape = shape();
        let build =
            |position| Vehicle::from_otp(&pattern(), &shape, position).expect("has coordinates");

        let mut first = position(Some(47.6), Some(-122.335));
        first.vehicle_id = Some("1:7204".to_owned());
        let mut second = position(Some(47.6), Some(-122.330));
        second.vehicle_id = Some("1:7205".to_owned());
        assert_ne!(build(first).id, build(second).id);

        // A feed that publishes no vehicle id still has to identify its buses somehow.
        let mut unnamed = position(Some(47.6), Some(-122.335));
        unnamed.vehicle_id = None;
        unnamed.label = Some("7204".to_owned());
        assert_eq!(build(unnamed).id, "1:40:0:01/7204");
    }

    /// The route rides along with each vehicle so a client needn't join back to the plan's legs.
    #[test]
    fn a_vehicle_carries_what_it_is_running() {
        let vehicle = Vehicle::from_otp(&pattern(), &shape(), position(Some(47.6), Some(-122.335)))
            .expect("has coordinates");

        let route = vehicle.route;
        assert_eq!(route.short_name.as_deref(), Some("40"));
        assert_eq!(route.color.as_deref(), Some("0080FF"));
        assert_eq!(vehicle.vehicle_mode, Some(gtfs_graphql::TransitMode::Bus));
        assert_eq!(vehicle.headsign.as_deref(), Some("Downtown Seattle"));
    }

    /// OTP reports a vehicle at exactly 0,0 for a run it hasn't started, reusing the id of the
    /// bus that will make it - so keeping it both draws a dot off West Africa and, sharing a
    /// marker with the real vehicle, drags the real one there too.
    #[test]
    fn a_vehicle_at_null_island_is_dropped() {
        let shape = shape();
        let build = |lat, lon| Vehicle::from_otp(&pattern(), &shape, position(lat, lon));

        assert!(build(Some(0.0), Some(0.0)).is_none());
        // Only exactly 0,0 - the Gulf of Guinea is a real place.
        assert!(build(Some(0.0), Some(-122.33)).is_some());
        assert!(build(Some(47.6), Some(0.0)).is_some());
    }

    #[test]
    fn a_vehicle_off_the_globe_is_dropped() {
        let shape = shape();
        let build = |lat, lon| Vehicle::from_otp(&pattern(), &shape, position(lat, lon));

        assert!(build(Some(91.0), Some(-122.33)).is_none());
        assert!(build(Some(47.6), Some(181.0)).is_none());
        assert!(build(Some(f64::NAN), Some(-122.33)).is_none());
        assert!(build(Some(47.6), Some(f64::INFINITY)).is_none());
    }

    /// Vehicles strung along the shape, identified by their longitude.
    fn vehicles_at(lons: &[f64]) -> Vec<Vehicle> {
        let shape = shape();
        lons.iter()
            .map(|lon| {
                let mut position = position(Some(47.600), Some(*lon));
                position.label = Some(format!("{lon}"));
                Vehicle::from_otp(&pattern(), &shape, position).expect("has coordinates")
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

        let nearby = nearby(vehicles, &shape, Some(boarding_stop));

        // Nearest first: the one just past the stop, then the two still approaching it.
        // -122.339 is a third vehicle still approaching, -122.325 and -122.305 are further past.
        assert_eq!(labels(&nearby), ["-122.329", "-122.332", "-122.336"]);
    }

    #[test]
    fn a_pattern_with_no_boarding_stop_reports_every_vehicle() {
        let shape = shape();
        let vehicles = vehicles_at(&[-122.339, -122.335, -122.331, -122.329, -122.305]);

        assert_eq!(nearby(vehicles, &shape, None).len(), 5);
    }

    #[test]
    fn fewer_vehicles_than_we_would_show_is_fine() {
        let shape = shape();
        let vehicles = vehicles_at(&[-122.335]);
        let nearby = nearby(vehicles, &shape, Some(Point::new(-122.330, 47.600)));

        assert_eq!(labels(&nearby), ["-122.335"]);
    }

    /// An undated report can't be aged or guessed forward, so there's nothing honest to draw.
    #[test]
    fn a_vehicle_that_cannot_date_its_report_is_dropped() {
        let mut position = position(Some(47.6), Some(-122.335));
        position.last_update = None;

        assert!(Vehicle::from_otp(&pattern(), &shape(), position).is_none());
    }

    #[test]
    fn a_vehicle_without_coordinates_is_dropped() {
        let shape = shape();
        let build = |lat, lon| Vehicle::from_otp(&pattern(), &shape, position(lat, lon));
        assert!(build(Some(47.6), Some(-122.33)).is_some());
        assert!(build(Some(47.6), None).is_none());
        assert!(build(None, None).is_none());
    }

    /// Written the same way as every other point v8 reports.
    #[test]
    fn a_vehicle_reports_its_position_as_a_lon_lat_pair() {
        let vehicle = Vehicle::from_otp(&pattern(), &shape(), position(Some(47.6), Some(-122.335)))
            .expect("has coordinates");
        let json = serde_json::to_value(&vehicle).expect("serializes");

        assert_eq!(json["position"], serde_json::json!([-122.335, 47.6]));
        assert!(json.get("lat").is_none());
    }

    #[test]
    fn an_undecodable_shape_is_dropped_rather_than_failing_the_request() {
        let undecodable = gtfs_graphql::PatternVehicles {
            geometry: Some("!!! not a polyline !!!".to_owned()),
            ..pattern()
        };
        assert!(shape_of(&undecodable).is_none());
    }

    /// `heading_for` is the longitude of the stop OTP says the vehicle is working towards.
    fn anchors_for(
        vehicle_lon: f64,
        reported_at: i64,
        heading_for: Option<f64>,
        stoptimes: &[VehicleStoptime],
    ) -> Vec<Anchor> {
        let shape = shape();
        let progress =
            progress_along(&shape, Point::new(vehicle_lon, 47.600)).expect("on the shape");
        let heading_for = heading_for.map(stop_id);
        anchors(
            &shape,
            progress,
            at(reported_at),
            heading_for.as_deref(),
            stoptimes,
        )
    }

    /// OTP hands back the whole day's stop sequence, most of which is behind the vehicle. The
    /// stop it says the vehicle is working towards is where the track starts.
    #[test]
    fn the_track_starts_at_the_stop_otp_says_the_vehicle_is_heading_for() {
        let stoptimes = [
            stoptime(-122.340, 100), // start of the line, well behind
            stoptime(-122.330, 200), // behind
            stoptime(-122.320, 400), // <- heading here
            stoptime(-122.310, 600),
        ];
        let anchors = anchors_for(-122.325, 300, Some(-122.320), &stoptimes);

        // The vehicle itself, then its next stop and the one after.
        assert_eq!(anchors.len(), 3);
        assert_eq!(anchors[1].time, at(400));
        assert_eq!(anchors[2].time, at(600));
    }

    /// The position and the predictions come out of the same minute-old snapshot, so a late run's
    /// next stop is routinely already overdue by the time we serve it. Scanning on to whichever
    /// prediction is still in the future finds a stop kilometres away with seconds left to reach
    /// it, and the vehicle gets flung down the route at several hundred metres a second.
    #[test]
    fn an_overdue_next_stop_yields_no_track_rather_than_a_leap() {
        let stoptimes = [
            stoptime(-122.339, 250), // the stop OTP says it's heading for - already overdue
            stoptime(-122.305, 320), // miles further on, and still in the future
        ];
        let anchors = anchors_for(-122.340, 300, Some(-122.339), &stoptimes);

        assert_eq!(anchors.len(), 1, "should not have anchored on the far stop");
        assert!(track(&shape(), &anchors).is_none());
    }

    /// Same shape of data, but the vehicle is where its next stop says it should be.
    #[test]
    fn a_next_stop_still_ahead_is_paced_normally() {
        let stoptimes = [stoptime(-122.320, 400), stoptime(-122.310, 600)];
        let anchors = anchors_for(-122.325, 300, Some(-122.320), &stoptimes);

        assert_eq!(anchors.len(), 3);
        assert_eq!(anchors[1].time, at(400));
    }

    /// Without a next stop there's no telling which predictions belong ahead of the vehicle.
    #[test]
    fn a_vehicle_with_no_stated_next_stop_gets_no_track() {
        let stoptimes = [stoptime(-122.320, 400), stoptime(-122.310, 600)];
        let anchors = anchors_for(-122.325, 300, None, &stoptimes);

        assert_eq!(anchors.len(), 1);
        assert!(track(&shape(), &anchors).is_none());
    }

    /// A vehicle standing at its next stop projects level with it. That's arrival, not a
    /// disagreement, so it should be paced on towards the stop after.
    #[test]
    fn a_vehicle_standing_at_its_next_stop_aims_at_the_one_after() {
        let stoptimes = [stoptime(-122.330, 400), stoptime(-122.320, 600)];
        // Reported a hair past the stop it's standing at.
        let anchors = anchors_for(-122.3299, 300, Some(-122.330), &stoptimes);

        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[1].time, at(600));
    }

    /// The position and the prediction are two different sources, and the first span is where
    /// they meet. A next stop that would need 200 m/s to reach is them contradicting each other.
    #[test]
    fn an_impossibly_fast_first_span_yields_no_track() {
        // 10km away, due in 50 seconds.
        let stoptimes = [stoptime(-122.220, 350), stoptime(-122.210, 900)];
        let anchors = anchors_for(-122.340, 300, Some(-122.220), &stoptimes);

        assert_eq!(anchors.len(), 1);
        assert!(track(&shape(), &anchors).is_none());
    }

    /// The same distance at a pace a vehicle could actually keep is fine.
    #[test]
    fn a_distant_next_stop_with_time_to_reach_it_is_kept() {
        let stoptimes = [stoptime(-122.220, 300 + 900)];
        let anchors = anchors_for(-122.340, 300, Some(-122.220), &stoptimes);

        assert_eq!(anchors.len(), 2);
    }

    /// A stop OTP names that isn't in the sequence we were given is no anchor either.
    #[test]
    fn an_unknown_next_stop_gets_no_track() {
        let stoptimes = [stoptime(-122.320, 400)];
        let anchors = anchors_for(-122.325, 300, Some(-122.999), &stoptimes);

        assert_eq!(anchors.len(), 1);
    }

    #[test]
    fn the_track_starts_where_the_vehicle_is_and_walks_toward_the_next_stop() {
        let stoptimes = [stoptime(-122.320, 400), stoptime(-122.310, 600)];
        let anchors = anchors_for(-122.325, 300, Some(-122.320), &stoptimes);
        let track = track(&shape(), &anchors).expect("has somewhere to go");

        assert!(
            track.points.len() > 2,
            "expected several samples, got {}",
            track.points.len()
        );
        // Evenly spaced in time, so a client indexes rather than searches. The track starts at
        // the vehicle's own lastUpdated, which is why it doesn't carry a start of its own.
        assert_eq!(anchors[0].time, at(300));
        assert_eq!(track.step_seconds, TRACK_STEP.num_seconds());
        // Running east, the way the shape does, and never past the last prediction.
        let lons: Vec<f64> = track.points.iter().copied().map(Point::x).collect();
        assert!(lons[1] > lons[0]);
        assert!(lons.windows(2).all(|pair| pair[1] >= pair[0]));
        let json = serde_json::to_value(&track).expect("serializes");
        assert_eq!(
            json["points"][0],
            serde_json::json!([track.points[0].x(), track.points[0].y()])
        );
        let ends_at =
            at(300) + TimeDelta::seconds(track.step_seconds * (track.points.len() as i64 - 1));
        assert!(ends_at <= at(600));
    }

    /// A trip predicted hours out shouldn't produce hours of guessed positions.
    #[test]
    fn the_track_stops_at_the_horizon() {
        let stoptimes = [stoptime(-122.310, 300 + 60 * 60)];
        let anchors = anchors_for(-122.325, 300, Some(-122.310), &stoptimes);
        let track = track(&shape(), &anchors).expect("has somewhere to go");

        let span = TimeDelta::seconds(track.step_seconds * (track.points.len() as i64 - 1));
        assert!(span <= TRACK_HORIZON, "track ran {span} past the horizon");
    }
}
