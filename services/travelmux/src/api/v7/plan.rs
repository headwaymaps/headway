use actix_web::web::{Data, Query};
use actix_web::{get, web, HttpRequest, HttpResponseBuilder};
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use geo::algorithm::BoundingRect;
use geo::geometry::{LineString, Point, Rect};
use polyline::decode_polyline;
use polyline::errors::PolylineError;
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::error::{PlanResponseErr, PlanResponseOk};
use super::TravelModes;
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql::{self, PlanDateTime};
use crate::util::format::format_meters;
use crate::util::haversine_segmenter::HaversineSegmenter;
use crate::util::serde_util::{
    deserialize_point_from_lat_lon, serialize_line_string_as_polyline6, serialize_rect_to_lng_lat,
};
use crate::util::{bearing_at_end, bearing_at_start, convert_to_meters, extend_bounds};
use crate::valhalla::valhalla_api;
use crate::valhalla::valhalla_api::{LonLat, ManeuverType};
use crate::{DistanceUnit, Error, TravelMode};

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanQuery {
    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    to_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    from_place: Point,

    num_itineraries: u32,

    mode: TravelModes,

    /// Which units the *prose* of an instruction is written in ("Continue for 2 miles.").
    ///
    /// Every numeric distance in the response is in meters regardless.
    preferred_distance_units: Option<DistanceUnit>,

    /// Desired departure (or arrival, if `arrive_by`) time. Either an RFC 3339 instant like
    /// "2024-06-13T14:30:00-07:00", or a wall clock time like "2024-06-13T14:30", which is
    /// interpreted in the timezone of the graph serving the trip.
    ///
    /// Only meaningful for transit trips. Defaults to now when omitted.
    date_time: Option<PlanDateTime>,

    /// When true, `date_time` describes the desired arrival time rather than departure time.
    #[serde(default)]
    arrive_by: bool,
}

impl PlanQuery {
    /// The units to write instruction prose in.
    fn instruction_units(&self) -> DistanceUnit {
        self.preferred_distance_units
            .unwrap_or(DistanceUnit::Kilometers)
    }
}

impl<'a> From<(&'a PlanQuery, Option<chrono_tz::Tz>)> for gtfs_graphql::PlanParams<'a> {
    /// The timezone isn't part of the client's query - it comes from the OTP graph serving the
    /// trip - but a local `date_time` is interpreted in it, so it's paired with the query here
    /// rather than left for the caller to remember.
    fn from((query, timezone): (&'a PlanQuery, Option<chrono_tz::Tz>)) -> Self {
        Self {
            from: query.from_place,
            to: query.to_place,
            modes: query.mode.as_slice(),
            num_itineraries: query.num_itineraries,
            date_time: query.date_time,
            arrive_by: query.arrive_by,
            timezone,
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    pub(crate) mode: TravelMode,

    /// RFC 3339, in the timezone of the graph that planned the trip
    pub(crate) start_time: DateTime<FixedOffset>,
    /// RFC 3339, in the timezone of the graph that planned the trip
    pub(crate) end_time: DateTime<FixedOffset>,

    pub(crate) duration_seconds: f64,
    pub(crate) distance_meters: f64,

    #[serde(serialize_with = "serialize_rect_to_lng_lat")]
    bounds: Rect,

    pub(crate) legs: Vec<Leg>,
}

impl Itinerary {
    pub fn combined_geometry(&self) -> LineString {
        let mut combined_geometry = LineString::new(vec![]);
        for leg in &self.legs {
            combined_geometry.0.extend(&leg.geometry.0);
        }
        combined_geometry
    }

    /// Valhalla has no notion of a departure time - it plans a trip of a given *length*, so we
    /// treat it as leaving now.
    pub fn from_valhalla(valhalla: &valhalla_api::Trip, mode: TravelMode) -> Self {
        let bounds = Rect::new(
            geo::coord!(x: valhalla.summary.min_lon, y: valhalla.summary.min_lat),
            geo::coord!(x: valhalla.summary.max_lon, y: valhalla.summary.max_lat),
        );

        debug_assert!(
            valhalla.locations.len() == valhalla.legs.len() + 1,
            "assuming each leg has a start and end location"
        );

        let itinerary_start_time = Utc::now().fixed_offset();
        let mut leg_start_time = itinerary_start_time;
        let legs = valhalla
            .legs
            .iter()
            .zip(valhalla.locations.windows(2))
            .map(|(v_leg, locations)| {
                let leg = Leg::from_valhalla(
                    v_leg,
                    mode,
                    leg_start_time,
                    locations[0],
                    locations[1],
                    valhalla.units,
                );
                leg_start_time = leg.end_time;
                leg
            })
            .collect();

        Self {
            mode,
            start_time: itinerary_start_time,
            end_time: itinerary_start_time + seconds(valhalla.summary.time),
            duration_seconds: valhalla.summary.time,
            distance_meters: convert_to_meters(valhalla.summary.length, valhalla.units),
            bounds,
            legs,
        }
    }

    pub fn from_otp(
        itinerary: gtfs_graphql::Itinerary,
        mode: TravelMode,
        instruction_units: DistanceUnit,
    ) -> crate::Result<Self> {
        let duration_seconds = itinerary.duration.unwrap_or(0).max(0) as f64;
        let otp_legs: Vec<gtfs_graphql::Leg> = itinerary.legs.into_iter().flatten().collect();
        let last_leg_idx = otp_legs.len().saturating_sub(1);
        let legs: Vec<Leg> = otp_legs
            .into_iter()
            .enumerate()
            .map(|(idx, leg)| Leg::from_otp(leg, idx == last_leg_idx, instruction_units))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e: PolylineError| Error::server(format!("failed to parse legs: {e}")))?;

        let mut legs_iter = legs.iter();
        let Some(first_leg) = legs_iter.next() else {
            return Err(Error::server("itinerary had no legs"));
        };
        let Some(mut itinerary_bounds) = first_leg.bounding_rect() else {
            return Err(Error::server("first leg has no bounding_rect"));
        };
        for leg in legs_iter {
            let Some(leg_bounds) = leg.bounding_rect() else {
                return Err(Error::server("leg has no bounding_rect"));
            };
            extend_bounds(&mut itinerary_bounds, &leg_bounds);
        }

        Ok(Self {
            mode,
            // OTP reports the itinerary's own start/end, but it's optional in the schema - the
            // legs, which aren't, say the same thing.
            start_time: itinerary.start.unwrap_or(first_leg.start_time),
            end_time: itinerary
                .end
                .unwrap_or_else(|| legs.last().expect("checked above").end_time),
            duration_seconds,
            distance_meters: legs.iter().map(|leg| leg.distance_meters).sum(),
            bounds: itinerary_bounds,
            legs,
        })
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Place {
    #[serde(flatten)]
    location: LonLat,
    /// Transit stops have names. Places the user picked usually don't.
    name: Option<String>,
}

impl From<&gtfs_graphql::Place> for Place {
    fn from(value: &gtfs_graphql::Place) -> Self {
        Self {
            location: LonLat {
                lat: value.lat,
                lon: value.lon,
            },
            name: value.name.clone(),
        }
    }
}

impl From<valhalla_api::LonLat> for Place {
    fn from(value: LonLat) -> Self {
        Self {
            location: value,
            name: None,
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Leg {
    /// encoded polyline. 1e-6 scale, (lat, lon)
    #[serde(serialize_with = "serialize_line_string_as_polyline6")]
    geometry: LineString,

    /// Which mode is this leg of the journey?
    pub(crate) mode: TravelMode,

    #[serde(flatten)]
    pub(crate) mode_leg: ModeLeg,

    /// Beginning of the leg
    from_place: Place,

    /// End of the leg
    to_place: Place,

    /// RFC 3339. Includes any real-time delay we know about.
    pub(crate) start_time: DateTime<FixedOffset>,

    /// RFC 3339. Includes any real-time delay we know about.
    pub(crate) end_time: DateTime<FixedOffset>,

    pub(crate) distance_meters: f64,

    pub(crate) duration_seconds: f64,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ModeLeg {
    #[serde(rename = "transitLeg")]
    Transit(Box<TransitLeg>),

    #[serde(rename = "nonTransitLeg")]
    NonTransit(Box<NonTransitLeg>),
}

/// A ride on a transit vehicle.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransitLeg {
    /// What kind of vehicle this is a ride on, e.g. `BUS` or `TRAM`, as OTP names it.
    ///
    /// The leg's `mode` is always `TRANSIT`; this is the detail behind it.
    vehicle_mode: Option<gtfs_graphql::Mode>,

    route: Option<Route>,

    /// The operator of the service, e.g. "Metro Transit"
    agency_name: Option<String>,

    /// What the vehicle displays at the boarding stop, e.g. "Downtown Seattle Via 35th Ave SW"
    headsign: Option<String>,

    /// Whether the leg's times reflect real-time data, rather than just the schedule.
    real_time: bool,

    alerts: Vec<Alert>,
}

impl From<&gtfs_graphql::Leg> for TransitLeg {
    fn from(leg: &gtfs_graphql::Leg) -> Self {
        Self {
            vehicle_mode: leg.mode.clone(),
            route: leg.route.as_ref().map(Route::from),
            agency_name: leg.agency.as_ref().map(|agency| agency.name.clone()),
            headsign: leg.headsign.clone(),
            real_time: leg.real_time.unwrap_or(false),
            alerts: leg
                .alerts
                .iter()
                .flatten()
                .flatten()
                .map(Alert::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// e.g. "40"
    short_name: Option<String>,
    /// e.g. "Downtown - Ballard"
    long_name: Option<String>,
    /// An RRGGBB hex color, without a leading "#"
    color: Option<String>,
}

impl From<&gtfs_graphql::Route> for Route {
    fn from(route: &gtfs_graphql::Route) -> Self {
        Self {
            short_name: route.short_name.clone(),
            long_name: route.long_name.clone(),
            color: route.color.clone(),
        }
    }
}

/// A service alert affecting a transit leg - a detour, an elevator outage, a cancelation.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    header_text: Option<String>,
    description_text: String,
    url: Option<String>,
    /// RFC 3339, UTC
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_start: Option<DateTime<Utc>>,
    /// RFC 3339, UTC
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_end: Option<DateTime<Utc>>,
}

impl From<&gtfs_graphql::Alert> for Alert {
    fn from(alert: &gtfs_graphql::Alert) -> Self {
        Self {
            header_text: alert.alert_header_text.clone(),
            description_text: alert.alert_description_text.clone(),
            url: alert.alert_url.clone(),
            // OTP gives these as Unix seconds
            effective_start: alert
                .effective_start_date
                .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
            effective_end: alert
                .effective_end_date
                .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NonTransitLeg {
    pub(crate) maneuvers: Vec<Maneuver>,

    /// The substantial road names along the route
    pub(crate) substantial_street_names: Vec<String>,
}

impl NonTransitLeg {
    fn new(maneuvers: Vec<Maneuver>) -> Self {
        let mut street_distances = HashMap::new();
        for maneuver in &maneuvers {
            if let Some(street_names) = &maneuver.street_names {
                for street_name in street_names {
                    *street_distances.entry(street_name).or_insert(0.0) += maneuver.distance_meters;
                }
            }
        }
        let mut scores: Vec<_> = street_distances.into_iter().collect();
        scores.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let limit = 3;
        // Don't include tiny segments in the description of the route
        let mut inclusion_threshold = None;

        let substantial_street_names = scores
            .into_iter()
            .take(limit)
            .flat_map(|(street_name, distance)| {
                let Some(inclusion_threshold) = inclusion_threshold else {
                    // don't consider streets that are much smaller than this one
                    inclusion_threshold = Some(distance * 0.5);
                    return Some(street_name.clone());
                };
                if distance > inclusion_threshold {
                    Some(street_name.clone())
                } else {
                    None
                }
            })
            .collect();

        Self {
            maneuvers,
            substantial_street_names,
        }
    }
}

/// One action taken by the user - like a turn or taking an exit.
/// This was originally based on the schema of a valhalla_api::Maneuver, but it can be built from
/// either OTP or Valhalla data.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Maneuver {
    pub instruction: Option<String>,
    #[serde(skip_serializing)]
    pub geometry: LineString,
    pub distance_meters: f64,
    pub street_names: Option<Vec<String>>,
    #[serde(skip_serializing)]
    pub duration_seconds: f64,
    pub r#type: ManeuverType,
    pub verbal_post_transition_instruction: Option<String>,
    pub start_point: LonLat,
    pub bearing_before: u16,
    pub bearing_after: u16,
}

impl Maneuver {
    fn from_valhalla(
        valhalla: valhalla_api::Maneuver,
        leg_geometry: &LineString,
        prev_maneuver_geometry: Option<&LineString>,
        units: DistanceUnit,
    ) -> Self {
        let coords = leg_geometry.0
            [valhalla.begin_shape_index as usize..=valhalla.end_shape_index as usize]
            .to_owned();
        let geometry = LineString::from(coords);

        let bearing_after = bearing_at_start(&geometry).unwrap_or(0);
        let bearing_before = prev_maneuver_geometry
            .and_then(bearing_at_end)
            .unwrap_or(bearing_after);

        Self {
            instruction: Some(valhalla.instruction),
            street_names: valhalla.street_names,
            duration_seconds: valhalla.time,
            r#type: valhalla.r#type,
            start_point: Point(leg_geometry[valhalla.begin_shape_index as usize]).into(),
            verbal_post_transition_instruction: valhalla.verbal_post_transition_instruction,
            distance_meters: convert_to_meters(valhalla.length, units),
            bearing_before,
            bearing_after,
            geometry,
        }
    }

    fn from_otp(
        otp: &gtfs_graphql::Step,
        geometry: LineString,
        prev_geometry: Option<&LineString>,
        leg: &gtfs_graphql::Leg,
        instruction_units: DistanceUnit,
    ) -> Self {
        let relative_direction = otp
            .relative_direction
            .unwrap_or(gtfs_graphql::RelativeDirection::Continue);
        let street_name = otp.street_name.clone().unwrap_or_default();
        let distance_meters = otp.distance.unwrap_or(0.0);

        let instruction = maneuver_instruction(
            leg.mode.as_ref(),
            relative_direction,
            otp.absolute_direction,
            &street_name,
        );

        let verbal_post_transition_instruction =
            build_verbal_post_transition_instruction(distance_meters, instruction_units);

        let street_names = if otp.bogus_name.unwrap_or(false) {
            None
        } else {
            Some(vec![street_name])
        };
        let bearing_after = bearing_at_start(&geometry).unwrap_or(0);
        let bearing_before = prev_geometry
            .and_then(bearing_at_end)
            .unwrap_or(bearing_after);

        // OTP times its legs but not the individual steps within them, so apportion the leg's
        // duration by how far each step goes.
        let leg_distance = leg.distance.unwrap_or(0.0);
        let duration_seconds = if leg_distance > 0.0 {
            distance_meters / leg_distance * leg_duration_seconds(leg)
        } else {
            0.0
        };

        Self {
            instruction,
            r#type: relative_direction.into(),
            street_names,
            verbal_post_transition_instruction,
            distance_meters,
            duration_seconds,
            start_point: LonLat {
                lat: otp.lat.unwrap_or(0.0),
                lon: otp.lon.unwrap_or(0.0),
            },
            bearing_before,
            bearing_after,
            geometry,
        }
    }
}

/// The duration of a leg, in seconds. OTP reports it directly, but it's nullable, in which case
/// the leg's own start and end times say the same thing.
fn leg_duration_seconds(leg: &gtfs_graphql::Leg) -> f64 {
    leg.duration.unwrap_or_else(|| {
        let elapsed = leg.end.best_estimate() - leg.start.best_estimate();
        elapsed.num_milliseconds() as f64 / 1000.0
    })
}

fn seconds(seconds: f64) -> TimeDelta {
    TimeDelta::milliseconds((seconds * 1000.0) as i64)
}

/// Returns the natural language description of the maneuver.
// We could do so much better. Look at Valhalla's Odin.
//
// e.g. take context of previous maneuver. "Bear right to stay on Main Street"
// TODO: localize
fn maneuver_instruction(
    mode: Option<&gtfs_graphql::Mode>,
    maneuver_type: gtfs_graphql::RelativeDirection,
    absolute_direction: Option<gtfs_graphql::AbsoluteDirection>,
    street_name: &str,
) -> Option<String> {
    use gtfs_graphql::{AbsoluteDirection, Mode, RelativeDirection};
    match maneuver_type {
        RelativeDirection::Depart => {
            if let Some(absolute_direction) = absolute_direction {
                let direction = match absolute_direction {
                    AbsoluteDirection::North => "north",
                    AbsoluteDirection::Northeast => "northeast",
                    AbsoluteDirection::East => "east",
                    AbsoluteDirection::Southeast => "southeast",
                    AbsoluteDirection::South => "south",
                    AbsoluteDirection::Southwest => "southwest",
                    AbsoluteDirection::West => "west",
                    AbsoluteDirection::Northwest => "northwest",
                };
                let mode = match mode {
                    Some(Mode::Walk) => "Walk",
                    Some(Mode::Bicycle) => "Bike",
                    Some(Mode::Car) => "Drive",
                    _ => "Transit",
                };
                Some(format!("{mode} {direction} on {street_name}."))
            } else {
                Some("Depart.".to_string())
            }
        }
        RelativeDirection::HardLeft => Some(format!("Turn sharp left onto {street_name}.")),
        RelativeDirection::Left => Some(format!("Turn left onto {street_name}.")),
        RelativeDirection::SlightlyLeft => Some(format!("Turn slightly left onto {street_name}.")),
        RelativeDirection::Continue => Some(format!("Continue onto {street_name}.")),
        RelativeDirection::SlightlyRight => {
            Some(format!("Turn slightly right onto {street_name}."))
        }
        RelativeDirection::Right => Some(format!("Turn right onto {street_name}.")),
        RelativeDirection::HardRight => Some(format!("Turn sharp right onto {street_name}.")),
        RelativeDirection::CircleClockwise | RelativeDirection::CircleCounterclockwise => {
            Some("Enter the roundabout.".to_string())
        }
        RelativeDirection::Elevator => Some("Enter the elevator.".to_string()),
        RelativeDirection::UturnLeft | RelativeDirection::UturnRight => {
            Some("Make a U-turn.".to_string())
        }
        // These only occur on transit legs (station entrances/exits, signage), for which we don't
        // render turn-by-turn maneuvers, so a generic instruction is fine.
        RelativeDirection::EnterStation
        | RelativeDirection::ExitStation
        | RelativeDirection::FollowSigns => Some("Continue.".to_string()),
    }
}

fn build_verbal_post_transition_instruction(
    distance_meters: f64,
    instruction_units: DistanceUnit,
) -> Option<String> {
    if distance_meters == 0.0 {
        None
    } else {
        Some(format!(
            "Continue for {}.",
            format_meters(distance_meters, instruction_units.measurement_system())
        ))
    }
}

impl Leg {
    pub(crate) const GEOMETRY_PRECISION: u32 = 6;
    const VALHALLA_GEOMETRY_PRECISION: u32 = 6;
    const OTP_GEOMETRY_PRECISION: u32 = 5;

    fn bounding_rect(&self) -> Option<Rect> {
        self.geometry.bounding_rect()
    }

    fn from_otp(
        otp: gtfs_graphql::Leg,
        is_destination_leg: bool,
        instruction_units: DistanceUnit,
    ) -> std::result::Result<Self, PolylineError> {
        debug_assert_ne!(Self::OTP_GEOMETRY_PRECISION, Self::GEOMETRY_PRECISION);
        let encoded_geometry = otp
            .leg_geometry
            .as_ref()
            .and_then(|geometry| geometry.points.as_deref())
            .unwrap_or_default();
        let geometry = decode_polyline(encoded_geometry, Self::OTP_GEOMETRY_PRECISION)?;
        let from_place: Place = (&otp.from).into();
        let to_place: Place = (&otp.to).into();
        let distance_meters = otp.distance.unwrap_or(0.0);

        let mode_leg = if otp.is_transit() {
            ModeLeg::Transit(Box::new(TransitLeg::from(&otp)))
        } else {
            let mut segmenter = HaversineSegmenter::new(geometry.clone());
            let steps = otp.steps.as_deref().unwrap_or_default();
            // +1 for the arrival maneuver we may synthesize below
            let mut maneuvers: Vec<Maneuver> = Vec::with_capacity(steps.len() + 1);
            for otp_step in steps.iter().flatten() {
                let step_geometry = segmenter
                    .next_segment(otp_step.distance.unwrap_or(0.0))
                    .unwrap_or_else(|| {
                        log::warn!("no geometry for step");
                        debug_assert!(false, "no geometry for step");
                        LineString::new(vec![])
                    });
                let prev_step_geometry = maneuvers
                    .last()
                    .map(|prev_maneuver: &Maneuver| &prev_maneuver.geometry);
                maneuvers.push(Maneuver::from_otp(
                    otp_step,
                    step_geometry,
                    prev_step_geometry,
                    &otp,
                    instruction_units,
                ));
            }

            // OTP doesn't include an arrival step like valhalla, so we synthesize one
            if is_destination_leg {
                let bearing_after = bearing_at_end(&geometry).unwrap_or(0);
                let bearing_before = maneuvers
                    .last()
                    .and_then(|maneuver| bearing_at_end(&maneuver.geometry))
                    .unwrap_or(bearing_after);
                maneuvers.push(Maneuver {
                    instruction: Some("Arrive at your destination.".to_string()),
                    distance_meters: 0.0,
                    street_names: None,
                    duration_seconds: 0.0,
                    r#type: ManeuverType::Destination,
                    verbal_post_transition_instruction: None,
                    start_point: to_place.location,
                    geometry: LineString::new(vec![to_place.location.into()]),
                    bearing_before,
                    bearing_after,
                });
            }

            ModeLeg::NonTransit(Box::new(NonTransitLeg::new(maneuvers)))
        };

        Ok(Self {
            from_place,
            to_place,
            start_time: otp.start.best_estimate(),
            end_time: otp.end.best_estimate(),
            duration_seconds: leg_duration_seconds(&otp),
            mode: otp
                .mode
                .as_ref()
                .map(Into::into)
                .unwrap_or(TravelMode::Transit),
            distance_meters,
            geometry,
            mode_leg,
        })
    }

    fn from_valhalla(
        valhalla: &valhalla_api::Leg,
        travel_mode: TravelMode,
        start_time: DateTime<FixedOffset>,
        from_place: LonLat,
        to_place: LonLat,
        units: DistanceUnit,
    ) -> Self {
        let geometry = decode_polyline(&valhalla.shape, Self::VALHALLA_GEOMETRY_PRECISION)
            .expect("valid polyline from valhalla");

        let mut maneuvers: Vec<Maneuver> = Vec::with_capacity(valhalla.maneuvers.len());
        for valhalla_maneuver in valhalla.maneuvers.iter().cloned() {
            let prev_maneuver_geometry = maneuvers
                .last()
                .map(|prev_maneuver| &prev_maneuver.geometry);
            let maneuver = Maneuver::from_valhalla(
                valhalla_maneuver,
                &geometry,
                prev_maneuver_geometry,
                units,
            );
            maneuvers.push(maneuver);
        }

        let leg = NonTransitLeg::new(maneuvers);
        Self {
            start_time,
            end_time: start_time + seconds(valhalla.summary.time),
            from_place: from_place.into(),
            to_place: to_place.into(),
            geometry,
            mode: travel_mode,
            mode_leg: ModeLeg::NonTransit(Box::new(leg)),
            distance_meters: convert_to_meters(valhalla.summary.length, units),
            duration_seconds: valhalla.summary.time,
        }
    }
}

impl actix_web::Responder for PlanResponseOk {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> actix_web::HttpResponse {
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        response.content_type("application/json");
        response.json(self)
    }
}

#[get("/v7/plan")]
pub async fn get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> std::result::Result<PlanResponseOk, PlanResponseErr> {
    _get_plan(query, req, app_state).await
}

pub async fn _get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> std::result::Result<PlanResponseOk, PlanResponseErr> {
    let Some(primary_mode) = query.mode.first() else {
        return Err(PlanResponseErr::from(Error::user("mode is required")));
    };

    // TODO: Handle bus+bike if bike is first, for now all our clients are responsible for enforcing that
    // the "primary" mode appears first.
    match primary_mode {
        TravelMode::Transit => otp_plan(&query, req, &app_state, primary_mode).await,
        other => {
            if primary_mode == &TravelMode::Bicycle || primary_mode == &TravelMode::Walk {
                match otp_plan(&query, req, &app_state, primary_mode).await {
                    Ok(otp_response) => {
                        debug_assert_eq!(
                            1,
                            otp_response.itineraries.len(),
                            "expected exactly one itinerary from OTP"
                        );
                        // Prefer OTP response when available - anecdotally, it tends to be higher quality than Valhalla routes for
                        // walking and cycling.
                        //
                        // We could combine the results and return them all, but I seemingly never want the valhalla directions when OTP are available.
                        //
                        // Plus, when re-routing, the navigation SDK tries to do route-matching so that the "most similar" route
                        // will be applied. The end result is that you sometimes end up on the valhalla route, which IME is typically worse.
                        return Ok(otp_response);
                    }
                    Err(e) => {
                        // match error_code to raw value of ErrorType enum
                        match ErrorType::try_from(e.error.error_code) {
                            Ok(ErrorType::NoCoverageForArea) => {
                                log::debug!("No OTP coverage for route");
                            }
                            other => {
                                debug_assert!(other.is_ok(), "unexpected error code: {e:?}");
                                // We're mixing with results from Valhalla anyway, so don't surface this error
                                // to the user. Likely we just don't support this area.
                                log::error!("OTP failed to plan {primary_mode:?} route: {e}");
                            }
                        }
                    }
                }
            }
            valhalla_plan(&query, &app_state, primary_mode, other).await
        }
    }
}

async fn valhalla_plan(
    query: &Query<PlanQuery>,
    app_state: &Data<AppState>,
    primary_mode: &TravelMode,
    other: &TravelMode,
) -> Result<PlanResponseOk, PlanResponseErr> {
    debug_assert!(query.mode.len() == 1, "valhalla only supports one mode");

    let mode = match other {
        TravelMode::Transit => unreachable!("handled above"),
        TravelMode::Bicycle => valhalla_api::ModeCosting::Bicycle,
        TravelMode::Car => valhalla_api::ModeCosting::Auto,
        TravelMode::Walk => valhalla_api::ModeCosting::Pedestrian,
    };

    // Valhalla writes its own instruction prose, so it's the one place the client's preferred
    // units matter to the request. We convert its distances to meters on the way out.
    let router_url = app_state.valhalla_router().plan_url(
        query.from_place,
        query.to_place,
        mode,
        query.num_itineraries,
        query.instruction_units(),
    )?;
    let valhalla_response: reqwest::Response = reqwest::get(router_url).await.map_err(|e| {
        log::error!("error while fetching from valhalla service: {e}");
        PlanResponseErr::from(Error::server(e))
    })?;
    if !valhalla_response.status().is_success() {
        log::warn!(
            "upstream HTTP Error from valhalla service: {}",
            valhalla_response.status()
        )
    }

    debug_assert_eq!(
        valhalla_response
            .headers()
            .get(HeaderName::from_static("content-type")),
        Some(&HeaderValue::from_str("application/json;charset=utf-8").unwrap())
    );

    let valhalla_route_response: valhalla_api::ValhallaRouteResponseResult =
        valhalla_response.json().await.map_err(|e| {
            log::error!("error while parsing valhalla response: {e}");
            PlanResponseErr::from(Error::server(e))
        })?;

    PlanResponseOk::from_valhalla(*primary_mode, valhalla_route_response)
}

async fn otp_plan(
    query: &web::Query<PlanQuery>,
    _req: HttpRequest,
    app_state: &web::Data<AppState>,
    primary_mode: &TravelMode,
) -> Result<PlanResponseOk, PlanResponseErr> {
    let (endpoint, timezone) = {
        let Some(router) = app_state
            .otp_cluster()
            .find_router(query.from_place, query.to_place)
        else {
            Err(
                Error::user("Transit directions not available for this area.")
                    .error_type(ErrorType::NoCoverageForArea),
            )?
        };
        (router.endpoint().clone(), router.timezone())
    };
    log::debug!("found matching router. Querying OTP GraphQL at: {endpoint}");

    let params = gtfs_graphql::PlanParams::from((&**query, timezone));

    let client = reqwest::Client::new();
    let plan = gtfs_graphql::plan_connection(&client, &endpoint, &params)
        .await
        .map_err(|e| {
            log::error!("error while fetching plan from otp service: {e}");
            PlanResponseErr::from(e)
        })?;

    PlanResponseOk::from_otp(*primary_mode, plan, query.instruction_units())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::gtfs_graphql::plan_result_from_fixture;
    use crate::valhalla::ValhallaRouter;
    use approx::assert_relative_eq;
    use geo::wkt;
    use serde_json::{json, Value};
    use std::fs::File;
    use std::io::BufReader;

    fn valhalla_response(name: &str) -> valhalla_api::ValhallaRouteResponseResult {
        let file = File::open(format!(
            "tests/fixtures/requests/valhalla_{name}_route.json"
        ))
        .unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(file)).unwrap();
        valhalla_api::ValhallaRouteResponseResult::Ok(valhalla)
    }

    fn otp_plan(name: &str) -> gtfs_graphql::PlanResult {
        plan_result_from_fixture(&format!(
            "tests/fixtures/requests/opentripplanner_{name}_planconnection.json"
        ))
    }

    #[test]
    fn parse_from_valhalla() {
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response("pedestrian"))
                .unwrap();
        assert_eq!(plan_response.itineraries.len(), 3);

        // itineraries
        let first_itinerary = &plan_response.itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Walk);
        // the fixture was requested in miles - 5.684 of them
        assert_relative_eq!(first_itinerary.distance_meters, 9147.48856);
        assert_relative_eq!(first_itinerary.duration_seconds, 6488.443);
        assert_eq!(
            first_itinerary.end_time - first_itinerary.start_time,
            seconds(6488.443)
        );
        assert_relative_eq!(
            first_itinerary.bounds,
            Rect::new(
                geo::coord!(x: -122.347201, y: 47.575663),
                geo::coord!(x: -122.335618, y: 47.651047)
            )
        );

        // legs
        assert_eq!(first_itinerary.legs.len(), 1);
        let first_leg = &first_itinerary.legs[0];
        assert_relative_eq!(
            first_leg.geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583),
            epsilon = 1e-4
        );

        assert_relative_eq!(
            geo::Point::from(first_leg.from_place.location),
            geo::point!(x: -122.339414, y: 47.575837)
        );
        assert_relative_eq!(
            geo::Point::from(first_leg.to_place.location),
            geo::point!(x:-122.347234, y: 47.651048)
        );
        assert!(first_leg.to_place.name.is_none());

        let ModeLeg::NonTransit(non_transit_leg) = &first_leg.mode_leg else {
            panic!("unexpected non-transit leg")
        };

        assert_eq!(first_leg.mode, TravelMode::Walk);
        assert_eq!(non_transit_leg.maneuvers.len(), 21);
    }

    /// The client's `preferredDistanceUnits` no longer affects any numeric field - everything is
    /// meters now - but it's still what the narrative instructions are written in, so trace it
    /// from the query string all the way to the prose.
    #[test]
    fn narrative_instructions_use_preferred_distance_units() {
        fn first_maneuver(query_string: &str) -> Maneuver {
            let query = Query::<PlanQuery>::from_query(query_string).unwrap();
            let plan_response = PlanResponseOk::from_otp(
                TravelMode::Walk,
                otp_plan("walk"),
                query.instruction_units(),
            )
            .unwrap();

            let first_leg = &plan_response.itineraries[0].legs[0];
            let ModeLeg::NonTransit(non_transit_leg) = &first_leg.mode_leg else {
                panic!("expected non-transit leg")
            };
            non_transit_leg.maneuvers[0].clone()
        }

        let base = "fromPlace=47.575837,-122.339414&toPlace=47.651048,-122.347234&numItineraries=1&mode=WALK";

        let imperial = first_maneuver(&format!("{base}&preferredDistanceUnits=miles"));
        assert_eq!(
            imperial.instruction.as_deref(),
            Some("Walk south on East Marginal Way South.")
        );
        assert_eq!(
            imperial.verbal_post_transition_instruction.as_deref(),
            Some("Continue for 60 feet.")
        );

        // Only the units of the prose change...
        let metric = first_maneuver(&format!("{base}&preferredDistanceUnits=kilometers"));
        assert_eq!(metric.instruction, imperial.instruction);
        assert_eq!(
            metric.verbal_post_transition_instruction.as_deref(),
            Some("Continue for 20 meters.")
        );

        // ...never the numbers, which are always meters.
        assert_relative_eq!(metric.distance_meters, imperial.distance_meters);
        assert_relative_eq!(metric.distance_meters, 19.15);

        // Omitting the param is allowed, and falls back to metric prose.
        let defaulted = first_maneuver(base);
        assert_eq!(
            defaulted.verbal_post_transition_instruction,
            metric.verbal_post_transition_instruction
        );
    }

    #[test]
    fn verbal_post_transition_instructions() {
        fn instruction(distance_meters: f64, units: DistanceUnit) -> Option<String> {
            build_verbal_post_transition_instruction(distance_meters, units)
        }

        assert_eq!(
            instruction(19.15, DistanceUnit::Meters).as_deref(),
            Some("Continue for 20 meters.")
        );
        assert_eq!(
            instruction(612.0, DistanceUnit::Kilometers).as_deref(),
            Some("Continue for 600 meters.")
        );
        // Metric prose graduates to kilometers on its own - the requested unit only picks the
        // measurement system.
        assert_eq!(
            instruction(1234.0, DistanceUnit::Meters).as_deref(),
            Some("Continue for 1.2 kilometers.")
        );

        assert_eq!(
            instruction(19.15, DistanceUnit::Miles).as_deref(),
            Some("Continue for 60 feet.")
        );
        assert_eq!(
            instruction(1234.0, DistanceUnit::Miles).as_deref(),
            Some("Continue for 0.8 miles.")
        );

        // A zero-length step has nothing to say.
        assert_eq!(instruction(0.0, DistanceUnit::Meters), None);
    }

    /// Valhalla is where the units actually leave travelmux - it writes the prose itself, so the
    /// client's preference has to make it into the upstream request, and whatever units come back
    /// have to be converted to meters on the way out.
    #[test]
    fn valhalla_instructions_use_preferred_distance_units() {
        fn requested_units(query_string: &str) -> String {
            let query = Query::<PlanQuery>::from_query(query_string).unwrap();
            let router = ValhallaRouter::new("http://valhalla:8002".parse().unwrap());
            let url = router
                .plan_url(
                    query.from_place,
                    query.to_place,
                    valhalla_api::ModeCosting::Pedestrian,
                    query.num_itineraries,
                    query.instruction_units(),
                )
                .unwrap();

            let (_, json) = url
                .query_pairs()
                .find(|(key, _)| key == "json")
                .expect("valhalla query is passed as json");
            let json: Value = serde_json::from_str(&json).unwrap();
            json["units"].as_str().unwrap().to_string()
        }

        let base = "fromPlace=47.575837,-122.339414&toPlace=47.651048,-122.347234&numItineraries=1&mode=WALK";
        assert_eq!(
            requested_units(&format!("{base}&preferredDistanceUnits=miles")),
            "miles"
        );
        assert_eq!(
            requested_units(&format!("{base}&preferredDistanceUnits=kilometers")),
            "kilometers"
        );
        // Omitting the param is allowed, and falls back to metric prose.
        assert_eq!(requested_units(base), "kilometers");

        // The prose valhalla wrote in those units comes back verbatim - this fixture was requested
        // in miles - while its numbers are converted to meters.
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response("pedestrian"))
                .unwrap();
        let first_leg = &plan_response.itineraries[0].legs[0];
        let ModeLeg::NonTransit(non_transit_leg) = &first_leg.mode_leg else {
            panic!("expected non-transit leg")
        };
        let first_maneuver = &non_transit_leg.maneuvers[0];
        assert_eq!(
            first_maneuver.instruction.as_deref(),
            Some("Walk south on East Marginal Way South.")
        );
        assert_eq!(
            first_maneuver.verbal_post_transition_instruction.as_deref(),
            Some("Continue for 60 feet.")
        );
        // 0.011 miles in the fixture
        assert_relative_eq!(first_maneuver.distance_meters, 17.70274);
    }

    #[test]
    fn parse_from_otp() {
        let plan_response = PlanResponseOk::from_otp(
            TravelMode::Transit,
            otp_plan("transit"),
            DistanceUnit::Miles,
        )
        .unwrap();

        let itineraries = plan_response.itineraries;
        assert_eq!(itineraries.len(), 6);

        // itineraries
        let first_itinerary = &itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Transit);
        assert_relative_eq!(first_itinerary.distance_meters, 10157.660000000002);
        assert_relative_eq!(first_itinerary.duration_seconds, 2347.0);
        // OTP reports its times in the graph's timezone, and we pass them through as-is.
        assert_eq!(
            first_itinerary.start_time.to_rfc3339(),
            "2024-05-17T12:35:01-07:00"
        );

        // legs
        assert_eq!(first_itinerary.legs.len(), 4);
        let first_leg = &first_itinerary.legs[0];
        assert_relative_eq!(
            first_leg.geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583),
            epsilon = 1e-4
        );

        assert_relative_eq!(
            geo::Point::from(first_leg.from_place.location),
            geo::point!(x: -122.339414, y: 47.575837)
        );
        assert_relative_eq!(
            geo::Point::from(first_leg.to_place.location),
            geo::point!(x: -122.334106, y: 47.575924)
        );
        assert_eq!(
            first_leg.to_place.name.as_ref().unwrap(),
            "1st Ave S & S Hanford St"
        );

        assert_eq!(first_leg.mode, TravelMode::Walk);
        let ModeLeg::NonTransit(non_transit_leg) = &first_leg.mode_leg else {
            panic!("expected non-transit leg")
        };
        let maneuvers = &non_transit_leg.maneuvers;
        assert_eq!(maneuvers.len(), 2);
        assert_eq!(maneuvers[0].r#type, ManeuverType::Start);
        assert_eq!(maneuvers[1].r#type, ManeuverType::Left);

        let transit_leg = &first_itinerary.legs[1];
        assert_eq!(transit_leg.mode, TravelMode::Transit);
        let ModeLeg::Transit(transit_leg) = &transit_leg.mode_leg else {
            panic!("expected transit leg")
        };
        assert_eq!(transit_leg.vehicle_mode, Some(gtfs_graphql::Mode::Bus));
        let route = transit_leg.route.as_ref().unwrap();
        assert_eq!(route.short_name.as_deref(), Some("21"));
        assert!(route.color.is_none());
        assert_eq!(transit_leg.agency_name.as_deref(), Some("Metro Transit"));
        assert!(!transit_leg.real_time);
    }

    #[test]
    fn serialize_response_from_otp() {
        let plan_response = PlanResponseOk::from_otp(
            TravelMode::Transit,
            otp_plan("transit"),
            DistanceUnit::Miles,
        )
        .unwrap();
        let response = serde_json::to_string(&plan_response).unwrap();
        let parsed_response: Value = serde_json::from_str(&response).unwrap();
        let first_itinerary = parsed_response
            .get("itineraries")
            .expect("field missing")
            .as_array()
            .unwrap()
            .first()
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.first().unwrap().as_object().unwrap();
        assert_eq!(first_leg.get("mode").unwrap().as_str().unwrap(), "WALK");
        assert_eq!(
            first_leg.get("startTime").expect("field missing"),
            "2024-05-17T12:35:01-07:00"
        );
        assert_eq!(
            first_leg.get("endTime").expect("field missing"),
            "2024-05-17T12:41:10-07:00"
        );

        assert!(first_leg.get("transitLeg").is_none());
        let non_transit_leg = first_leg.get("nonTransitLeg").unwrap().as_object().unwrap();

        let substantial_street_names = non_transit_leg
            .get("substantialStreetNames")
            .unwrap()
            .as_array()
            .unwrap();
        let expected_names = vec!["East Marginal Way South"];
        assert_eq!(substantial_street_names, &expected_names);

        let maneuvers = non_transit_leg
            .get("maneuvers")
            .unwrap()
            .as_array()
            .unwrap();
        let first_maneuver = maneuvers.first().unwrap();
        let expected_maneuver = json!({
            "bearingAfter": 182,
            "bearingBefore": 182,
            "distanceMeters": 19.15,
            "instruction": "Walk south on East Marginal Way South.",
            "startPoint": {
                "lat": 47.5758346,
                "lon": -122.3392181
            },
            "streetNames": ["East Marginal Way South"],
            "type": 1,
            "verbalPostTransitionInstruction": "Continue for 60 feet."
        });
        assert_eq!(first_maneuver, &expected_maneuver);

        // Verify transit leg
        let transit_leg = legs.get(1).unwrap().as_object().unwrap();
        assert_eq!(
            transit_leg.get("mode").unwrap().as_str().unwrap(),
            "TRANSIT"
        );
        assert!(transit_leg.get("nonTransitLeg").is_none());
        let transit_leg = transit_leg
            .get("transitLeg")
            .unwrap()
            .as_object()
            .expect("json object");

        // Brittle: If the fixtures are updated, these values might change due to time of day or whatever.
        assert_eq!(
            transit_leg.get("agencyName").unwrap().as_str().unwrap(),
            "Metro Transit"
        );
        assert_eq!(
            transit_leg.get("vehicleMode").unwrap().as_str().unwrap(),
            "BUS"
        );
        assert_eq!(
            transit_leg.get("route").unwrap().get("shortName").unwrap(),
            "21"
        );

        let alerts = transit_leg.get("alerts").unwrap().as_array().unwrap();
        let first_alert = alerts.first().unwrap().as_object().unwrap();
        // OTP dates its alerts in unix seconds; we hand clients a timestamp like everything else.
        assert_eq!(
            first_alert.get("effectiveStart").unwrap(),
            "2024-03-31T22:30:00Z"
        );
        assert!(first_alert
            .get("headerText")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("Route 21"));
    }

    #[test]
    fn serialize_response_from_valhalla() {
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response("pedestrian"))
                .unwrap();

        let response = serde_json::to_string(&plan_response).unwrap();
        let parsed_response: Value = serde_json::from_str(&response).unwrap();
        let first_itinerary = parsed_response
            .get("itineraries")
            .unwrap()
            .as_array()
            .unwrap()
            .first()
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.first().unwrap().as_object().unwrap();
        assert_eq!(first_leg.get("mode").unwrap().as_str().unwrap(), "WALK");
        assert!(first_leg.get("transitLeg").is_none());
        let non_transit_leg = first_leg.get("nonTransitLeg").unwrap().as_object().unwrap();

        let substantial_street_names = non_transit_leg
            .get("substantialStreetNames")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(
            substantial_street_names,
            &[
                "Dexter Avenue",
                "East Marginal Way South",
                "Alaskan Way South"
            ]
        );

        let maneuvers = non_transit_leg
            .get("maneuvers")
            .unwrap()
            .as_array()
            .unwrap();
        let first_maneuver = maneuvers.first().unwrap();
        let expected_maneuver = json!({
            "bearingAfter": 180,
            "bearingBefore": 180,
            "type": 2,
            "instruction": "Walk south on East Marginal Way South.",
            "verbalPostTransitionInstruction": "Continue for 60 feet.",
            // 0.011 miles, as valhalla reported it
            "distanceMeters": 17.70274,
            "startPoint": {
                "lat": 47.575836,
                "lon": -122.339216
            },
            "streetNames": ["East Marginal Way South"],
        });
        assert_eq!(first_maneuver, &expected_maneuver);
    }

    #[test]
    fn parse_maneuver_from_valhalla_json() {
        // deserialize a maneuver from a JSON string
        let json = r#"
        {
            "begin_shape_index": 0,
            "cost": 246.056,
            "end_shape_index": 1,
            "highway": true,
            "instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "length": 2.218,
            "street_names": [
            "Fauntleroy Way Southwest"
            ],
            "time": 198.858,
            "travel_mode": "drive",
            "travel_type": "car",
            "type": 2,
            "verbal_post_transition_instruction": "Continue for 2 miles.",
            "verbal_pre_transition_instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "verbal_succinct_transition_instruction": "Drive northeast."
        }"#;

        let valhalla_maneuver: valhalla_api::Maneuver = serde_json::from_str(json).unwrap();
        assert_eq!(valhalla_maneuver.r#type, ManeuverType::StartRight);
        assert_eq!(
            valhalla_maneuver.instruction,
            "Drive northeast on Fauntleroy Way Southwest."
        );

        // fake geometry
        let leg_geometry = wkt!(LINESTRING(-122.398 47.564,-122.396 47.566));
        let maneuver = Maneuver::from_valhalla(
            valhalla_maneuver,
            &leg_geometry,
            None,
            DistanceUnit::Kilometers,
        );
        let actual: Value =
            serde_json::from_str(&serde_json::to_string(&maneuver).unwrap()).unwrap();

        let expected = json!({
            "bearingAfter": 34,
            "bearingBefore": 34,
            "distanceMeters": 2218.0,
            "instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "type": 2,
            "startPoint": { "lon": -122.398, "lat": 47.564},
            "streetNames": ["Fauntleroy Way Southwest"],
            "verbalPostTransitionInstruction": "Continue for 2 miles.",
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn valhalla_error_becomes_plan_error() {
        let json = serde_json::json!({
            "error_code": 154,
            "error": "Path distance exceeds the max distance limit: 200000 meters",
            "status_code": 400,
            "status": "Bad Request"
        })
        .to_string();

        let valhalla_error: valhalla_api::RouteResponseError = serde_json::from_str(&json).unwrap();
        let plan_error = PlanResponseErr::from(valhalla_error);
        assert_eq!(plan_error.error.status_code, 400);
        assert_eq!(plan_error.error.error_code, 2154);
    }

    #[test]
    fn otp_routing_error_becomes_plan_error() {
        let body = json!({
            "data": { "planConnection": {
                "edges": [],
                "routingErrors": [ { "code": "OUTSIDE_BOUNDS", "description": "Origin is outside of the coverage area." } ]
            } }
        });
        let plan = crate::otp::gtfs_graphql::plan_result_from_json(body);

        let error = PlanResponseOk::from_otp(TravelMode::Transit, plan, DistanceUnit::Miles)
            .expect_err("expected an error");
        assert_eq!(error.error.status_code, 400);
        // Out-of-bounds is how a caller knows to try another router
        assert_eq!(error.error.error_code, ErrorType::NoCoverageForArea as u32);
        assert!(error.error.message.contains("OUTSIDE_BOUNDS"));
    }

    #[test]
    fn maneuver_bearing() {
        let a = wkt!(LINESTRING(0. 0.,1. 0.,1. 1.));
        let b = wkt!(LINESTRING(1. 1., 0. 1., 0. 0.));

        assert_eq!(90, bearing_at_start(&a).unwrap());
        assert_eq!(0, bearing_at_end(&a).unwrap());

        assert_eq!(270, bearing_at_start(&b).unwrap());
        assert_eq!(180, bearing_at_end(&b).unwrap());
    }
}
