use actix_web::{get, web, HttpRequest, HttpResponseBuilder};
use geo::algorithm::BoundingRect;
use geo::geometry::{LineString, Point, Rect};
use polyline::decode_polyline;
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::error::{PlanResponseErr, PlanResponseOk};
use super::TravelModes;

use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::otp_api;
use crate::otp::otp_api::{AbsoluteDirection, RelativeDirection};
use crate::util::format::format_meters;
use crate::util::{
    deserialize_point_from_lat_lon, extend_bounds, serialize_line_string_as_polyline6,
    serialize_rect_to_lng_lat, serialize_system_time_as_millis, system_time_from_millis,
};
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

    /// Ignored by OTP - transit trips will always be metric.
    /// Examine the `distance_units` in the response `Itinerary` to correctly interpret the response.
    preferred_distance_units: Option<DistanceUnit>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub(crate) itineraries: Vec<Itinerary>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    mode: TravelMode,
    /// seconds
    duration: f64,
    /// unix millis, UTC
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    start_time: SystemTime,
    /// unix millis, UTC
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    end_time: SystemTime,
    /// Units are in `distance_units`
    distance: f64,
    /// FIXME: I think we're returning meters even though distance unit is "Kilometers"
    /// Probably we should rename DistanceUnit::Kilometers to DistanceUnit::Meters
    /// This is passed as a parameter though, so it'd be a breaking change.
    distance_units: DistanceUnit,
    #[serde(serialize_with = "serialize_rect_to_lng_lat")]
    bounds: Rect,
    legs: Vec<Leg>,
}

impl Itinerary {
    pub fn distance_meters(&self) -> f64 {
        match self.distance_units {
            DistanceUnit::Kilometers => self.distance,
            DistanceUnit::Miles => self.distance * 1609.34,
        }
    }

    pub fn combined_geometry(&self) -> LineString {
        let mut combined_geometry = LineString::new(vec![]);
        for leg in &self.legs {
            combined_geometry.0.extend(&leg.geometry.0);
        }
        combined_geometry
    }

    pub fn from_valhalla(valhalla: &valhalla_api::Trip, mode: TravelMode) -> Self {
        let bounds = Rect::new(
            geo::coord!(x: valhalla.summary.min_lon, y: valhalla.summary.min_lat),
            geo::coord!(x: valhalla.summary.max_lon, y: valhalla.summary.max_lat),
        );

        let start_time = SystemTime::now();
        let end_time = start_time + Duration::from_millis((valhalla.summary.time * 1000.0) as u64);
        debug_assert!(
            valhalla.locations.len() == valhalla.legs.len() + 1,
            "assuming each leg has a start and end location"
        );

        let mut start_time = SystemTime::now();
        let legs = valhalla
            .legs
            .iter()
            .zip(valhalla.locations.windows(2))
            .map(|(v_leg, locations)| {
                let leg_start_time = start_time;
                let leg_end_time =
                    start_time + Duration::from_millis((v_leg.summary.time * 1000.0) as u64);
                start_time = leg_end_time;
                Leg::from_valhalla(
                    v_leg,
                    mode,
                    leg_start_time,
                    leg_end_time,
                    locations[0],
                    locations[1],
                )
            })
            .collect();

        Self {
            mode,
            start_time,
            end_time,
            duration: valhalla.summary.time,
            distance: valhalla.summary.length,
            bounds,
            distance_units: valhalla.units,
            legs,
        }
    }

    pub fn from_otp(
        itinerary: &otp_api::Itinerary,
        mode: TravelMode,
        distance_unit: DistanceUnit,
    ) -> crate::Result<Self> {
        // OTP responses are always in meters
        let distance_meters: f64 = itinerary.legs.iter().map(|l| l.distance).sum();
        let Ok(legs): std::result::Result<Vec<_>, _> = itinerary
            .legs
            .iter()
            .enumerate()
            .map(|(idx, leg)| {
                let is_destination_leg = idx == itinerary.legs.len() - 1;
                Leg::from_otp(leg, is_destination_leg, distance_unit)
            })
            .collect()
        else {
            return Err(Error::server("failed to parse legs"));
        };

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
            duration: itinerary.duration as f64,
            start_time: system_time_from_millis(itinerary.start_time),
            end_time: system_time_from_millis(itinerary.end_time),
            mode,
            distance: distance_meters / 1000.0,
            distance_units: DistanceUnit::Kilometers,
            bounds: itinerary_bounds,
            legs,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Place {
    #[serde(flatten)]
    location: LonLat,
    name: Option<String>,
}

impl From<&otp_api::Place> for Place {
    fn from(value: &otp_api::Place) -> Self {
        Self {
            location: value.location.into(),
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
struct Leg {
    /// encoded polyline. 1e-6 scale, (lat, lon)
    #[serde(serialize_with = "serialize_line_string_as_polyline6")]
    geometry: LineString,

    /// Which mode is this leg of the journey?
    mode: TravelMode,

    #[serde(flatten)]
    mode_leg: ModeLeg,

    /// Beginning of the leg
    from_place: Place,

    /// End of the Leg
    to_place: Place,

    // This is mostly OTP specific. We can synthesize a value from the valhalla response, but we
    // don't currently use it.
    /// Start time of the leg
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    start_time: SystemTime,

    // This is mostly OTP specific. We can synthesize a value from the valhalla response, but we
    // don't currently use it.
    /// Start time of the leg
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    end_time: SystemTime,

    /// Length of this leg
    distance_meters: f64,

    /// Duration of this leg
    duration_seconds: f64,
}

// Should we just pass the entire OTP leg?
type TransitLeg = otp_api::Leg;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ModeLeg {
    // REVIEW: rename? There is a boolean field for OTP called TransitLeg
    #[serde(rename = "transitLeg")]
    Transit(Box<TransitLeg>),

    #[serde(rename = "nonTransitLeg")]
    NonTransit(Box<NonTransitLeg>),
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NonTransitLeg {
    maneuvers: Vec<Maneuver>,

    /// The substantial road names along the route
    substantial_street_names: Vec<String>,
}

impl NonTransitLeg {
    fn new(maneuvers: Vec<Maneuver>) -> Self {
        let mut street_distances = HashMap::new();
        for maneuver in &maneuvers {
            if let Some(street_names) = &maneuver.street_names {
                for street_name in street_names {
                    *street_distances.entry(street_name).or_insert(0.0) += maneuver.distance;
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

// Eventually we might want to coalesce this into something not valhalla specific
// but for now we only use it for valhalla trips
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Maneuver {
    pub instruction: Option<String>,
    // pub cost: f64,
    // pub begin_shape_index: u64,
    // pub end_shape_index: u64,
    #[serde(skip_serializing)]
    pub geometry: LineString,
    // pub highway: Option<bool>,
    /// In units of the `distance_unit` of the trip leg
    pub distance: f64,
    pub street_names: Option<Vec<String>>,
    #[serde(skip_serializing)]
    pub duration_seconds: f64,
    // pub travel_mode: String,
    // pub travel_type: String,
    pub r#type: ManeuverType,
    pub verbal_post_transition_instruction: Option<String>,
    pub start_point: LonLat,
    // pub verbal_pre_transition_instruction: Option<String>,
    // pub verbal_succinct_transition_instruction: Option<String>,
}

impl Maneuver {
    fn from_valhalla(valhalla: valhalla_api::Maneuver, leg_geometry: &LineString) -> Self {
        let coords = leg_geometry.0
            [valhalla.begin_shape_index as usize..=valhalla.end_shape_index as usize]
            .to_owned();
        let geometry = LineString::from(coords);
        Self {
            instruction: Some(valhalla.instruction),
            street_names: valhalla.street_names,
            duration_seconds: valhalla.time,
            r#type: valhalla.r#type,
            start_point: Point(leg_geometry[valhalla.begin_shape_index as usize]).into(),
            verbal_post_transition_instruction: valhalla.verbal_post_transition_instruction,
            distance: valhalla.length,
            geometry,
        }
    }

    fn from_otp(
        otp: otp_api::Step,
        mode: otp_api::TransitMode,
        distance_unit: DistanceUnit,
        // leg_geometry: &LineString,
    ) -> Self {
        let instruction = build_instruction(
            mode,
            otp.relative_direction,
            otp.absolute_direction,
            &otp.street_name,
        );

        let verbal_post_transition_instruction =
            build_verbal_post_transition_instruction(otp.distance, distance_unit);

        let street_names = if let Some(true) = otp.bogus_name {
            None
        } else {
            Some(vec![otp.street_name])
        };
        let localized_distance = match distance_unit {
            DistanceUnit::Kilometers => otp.distance,
            // round to the nearest ten-thousandth
            DistanceUnit::Miles => (otp.distance * 0.621371 * 10_000.0).round() / 10_000.0,
        };

        log::error!("TODO: synthesize geometry for OTP steps");
        let geometry = LineString::new(vec![]);
        Self {
            instruction,
            r#type: otp.relative_direction.into(),
            street_names,
            verbal_post_transition_instruction,
            distance: localized_distance,
            duration_seconds: 666.0, // TODO: OTP doesn't provide this at a granular level - only at the Leg level
            start_point: Point::new(otp.lon, otp.lat).into(),
            geometry,
        }
    }

    fn distance_meters(&self, distance_unit: DistanceUnit) -> f64 {
        match distance_unit {
            DistanceUnit::Kilometers => self.distance,
            DistanceUnit::Miles => self.distance * 1609.34,
        }
    }
}

// We could do so much better. Look at Valhalla's Odin.
//
// e.g. take context of previous maneuver. "Bear right to stay on Main Street"
fn build_instruction(
    mode: otp_api::TransitMode,
    maneuver_type: otp_api::RelativeDirection,
    absolute_direction: Option<otp_api::AbsoluteDirection>,
    street_name: &str,
) -> Option<String> {
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
                    otp_api::TransitMode::Walk => "Walk",
                    otp_api::TransitMode::Bicycle => "Bike",
                    otp_api::TransitMode::Car => "Drive",
                    _ => "Transit",
                };
                Some(format!("{mode} {direction} on {street_name}."))
            } else {
                Some("Depart.".to_string())
            }
        }
        RelativeDirection::HardLeft => Some(format!("Turn left onto {street_name}.")),
        RelativeDirection::Left => Some(format!("Turn left onto {street_name}.")),
        RelativeDirection::SlightlyLeft => Some(format!("Turn slightly left onto {street_name}.")),
        RelativeDirection::Continue => Some(format!("Continue onto {street_name}.")),
        RelativeDirection::SlightlyRight => {
            Some(format!("Turn slightly right onto {street_name}."))
        }
        RelativeDirection::Right => Some(format!("Turn right onto {street_name}.")),
        RelativeDirection::HardRight => Some(format!("Turn right onto {street_name}.")),
        RelativeDirection::CircleClockwise | RelativeDirection::CircleCounterclockwise => {
            Some("Enter the roundabout.".to_string())
        }
        RelativeDirection::Elevator => Some("Enter the elevator.".to_string()),
        RelativeDirection::UturnLeft | RelativeDirection::UturnRight => {
            Some("Make a U-turn.".to_string())
        }
    }
}

fn build_verbal_post_transition_instruction(
    distance: f64,
    distance_unit: DistanceUnit,
) -> Option<String> {
    if distance == 0.0 {
        None
    } else {
        Some(format!(
            "Continue for {}.",
            format_meters(distance, distance_unit)
        ))
    }
}

impl Leg {
    const GEOMETRY_PRECISION: u32 = 6;
    const VALHALLA_GEOMETRY_PRECISION: u32 = 6;
    const OTP_GEOMETRY_PRECISION: u32 = 5;

    fn bounding_rect(&self) -> Option<Rect> {
        self.geometry.bounding_rect()
    }

    fn from_otp(
        otp: &otp_api::Leg,
        is_destination_leg: bool,
        distance_unit: DistanceUnit,
    ) -> std::result::Result<Self, String> {
        debug_assert_ne!(Self::OTP_GEOMETRY_PRECISION, Self::GEOMETRY_PRECISION);
        let geometry = decode_polyline(&otp.leg_geometry.points, Self::OTP_GEOMETRY_PRECISION)?;
        let from_place: Place = (&otp.from).into();
        let to_place: Place = (&otp.to).into();

        let mode_leg = match otp.mode {
            otp_api::TransitMode::Walk
            | otp_api::TransitMode::Bicycle
            | otp_api::TransitMode::Car => {
                let mut maneuvers: Vec<_> = otp
                    .steps
                    .iter()
                    .cloned()
                    .map(|otp_step| Maneuver::from_otp(otp_step, otp.mode, distance_unit))
                    .collect();

                // OTP doesn't include an arrival step like valhalla, so we synthesize one
                if is_destination_leg {
                    let maneuver = Maneuver {
                        instruction: Some("Arrive at your destination.".to_string()),
                        distance: 0.0,
                        street_names: None,
                        duration_seconds: 0.0,
                        r#type: ManeuverType::Destination,
                        verbal_post_transition_instruction: None,
                        start_point: to_place.location,
                        geometry: LineString::new(vec![to_place.location.into()]),
                    };
                    maneuvers.push(maneuver);
                }
                let leg = NonTransitLeg::new(maneuvers);
                ModeLeg::NonTransit(Box::new(leg))
            }
            _ => {
                // assume everything else is transit
                ModeLeg::Transit(Box::new(otp.clone()))
            }
        };

        Ok(Self {
            from_place,
            to_place,
            start_time: system_time_from_millis(otp.start_time),
            end_time: system_time_from_millis(otp.end_time),
            geometry,
            mode: otp.mode.into(),
            distance_meters: otp.distance,
            duration_seconds: (otp.end_time - otp.start_time) as f64 / 1000.0,
            mode_leg,
        })
    }

    fn from_valhalla(
        valhalla: &valhalla_api::Leg,
        travel_mode: TravelMode,
        start_time: SystemTime,
        end_time: SystemTime,
        from_place: LonLat,
        to_place: LonLat,
    ) -> Self {
        let geometry =
            polyline::decode_polyline(&valhalla.shape, Self::VALHALLA_GEOMETRY_PRECISION)
                .expect("valid polyline from valhalla");
        let maneuvers = valhalla
            .maneuvers
            .iter()
            .cloned()
            .map(|valhalla_maneuver| Maneuver::from_valhalla(valhalla_maneuver, &geometry))
            .collect();
        let leg = NonTransitLeg::new(maneuvers);
        Self {
            start_time,
            end_time,
            from_place: from_place.into(),
            to_place: to_place.into(),
            geometry,
            mode: travel_mode,
            mode_leg: ModeLeg::NonTransit(Box::new(leg)),
            // TODO: verify units here - might be in miles
            distance_meters: valhalla.summary.length,
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

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DirectionsResponseOk {
    routes: Vec<osrm_api::Route>,
}

impl actix_web::Responder for DirectionsResponseOk {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> actix_web::HttpResponse {
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        response.content_type("application/json");
        response.json(self)
    }
}

#[get("/v5/directions")]
pub async fn get_directions(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> std::result::Result<DirectionsResponseOk, PlanResponseErr> {
    let plan_response_ok = _get_plan(query, req, app_state).await?;
    Ok(plan_response_ok.into())
}

pub mod osrm_api {
    use super::{Itinerary, Leg, Maneuver, ModeLeg};
    use crate::otp::otp_api::RelativeDirection::Continue;
    use crate::util::{serialize_line_string_as_polyline6, serialize_point_as_lon_lat_pair};
    use crate::valhalla::valhalla_api::ManeuverType;
    use crate::{DistanceUnit, TravelMode};
    use geo::{LineString, Point};
    use serde::Serialize;

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Route {
        /// The distance traveled by the route, in float meters.
        pub distance: f64,

        /// The estimated travel time, in float number of seconds.
        pub duration: f64,

        // todo: simplify?
        /// The entire geometry of the route
        #[serde(serialize_with = "serialize_line_string_as_polyline6")]
        pub geometry: LineString,

        /// The legs between the given waypoints
        pub legs: Vec<RouteLeg>,
    }

    impl From<Itinerary> for Route {
        fn from(itinerary: Itinerary) -> Self {
            Route {
                distance: itinerary.distance_meters(),
                duration: itinerary.duration,
                geometry: itinerary.combined_geometry(),
                legs: itinerary
                    .legs
                    .into_iter()
                    .map(|leg| RouteLeg::from_leg(leg, itinerary.distance_units))
                    .collect(),
            }
        }
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RouteLeg {
        /// The distance traveled by this leg, in float meters.
        pub distance: f64,

        /// The estimated travel time, in float number of seconds.
        pub duration: f64,

        /// A short human-readable summary of the route leg
        pub summary: String,

        /// Objects describing the turn-by-turn instructions of the route leg
        pub steps: Vec<RouteStep>,
        // /// Additional details about each coordinate along the route geometry
        // annotation: Annotation
    }

    impl RouteLeg {
        fn from_leg(value: Leg, distance_unit: DistanceUnit) -> Self {
            let (summary, steps) = match value.mode_leg {
                ModeLeg::Transit(_) => {
                    debug_assert!(
                        false,
                        "didn't expect to generate navigation for transit leg"
                    );
                    ("".to_string(), vec![])
                }
                ModeLeg::NonTransit(non_transit_leg) => {
                    let summary = non_transit_leg.substantial_street_names.join(", ");
                    let steps = non_transit_leg
                        .maneuvers
                        .into_iter()
                        .map(|maneuver| {
                            RouteStep::from_maneuver(maneuver, value.mode, distance_unit)
                        })
                        .collect();
                    (summary, steps)
                }
            };
            Self {
                distance: value.distance_meters,
                duration: value.duration_seconds,
                summary,
                steps,
            }
        }
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RouteStep {
        /// The distance traveled by this step, in float meters.
        pub distance: f64,

        /// The estimated travel time, in float number of seconds.
        pub duration: f64,

        /// The unsimplified geometry of the route segment.
        #[serde(serialize_with = "serialize_line_string_as_polyline6")]
        pub geometry: LineString,

        /// The name of the way along which travel proceeds.
        pub name: String,

        /// A reference number or code for the way. Optionally included, if ref data is available for the given way.
        pub r#ref: Option<String>,

        /// The pronunciation hint of the way name. Will be undefined if there is no pronunciation hit.
        pub pronunciation: Option<String>,

        /// The destinations of the way. Will be undefined if there are no destinations.
        pub destinations: Option<Vec<String>>,

        /// A string signifying the mode of transportation.
        pub mode: TravelMode,

        /// A `StepManeuver` object representing the maneuver.
        pub maneuver: StepManeuver,

        /// A list of `BannerInstruction` objects that represent all signs on the step.
        pub banner_instructions: Option<Vec<BannerInstruction>>,

        /// A list of `Intersection` objects that are passed along the segment, the very first belonging to the `StepManeuver`
        pub intersections: Option<Vec<Intersection>>,
    }

    impl RouteStep {
        fn from_maneuver(
            value: Maneuver,
            mode: TravelMode,
            from_distance_unit: DistanceUnit,
        ) -> Self {
            let banner_instructions = BannerInstruction::from_maneuver(&value);
            RouteStep {
                distance: value.distance_meters(from_distance_unit),
                duration: value.duration_seconds,
                geometry: value.geometry,
                name: value
                    .street_names
                    .unwrap_or(vec!["".to_string()])
                    .join(", "),
                r#ref: None,
                pronunciation: None,
                destinations: None,
                mode,
                maneuver: StepManeuver {
                    location: value.start_point.into(),
                },
                intersections: None, //vec![],
                banner_instructions,
            }
        }
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct BannerInstruction {
        distance_along_geometry: f64,
        primary: BannerInstructionContent,
        // secondary: Option<BannerInstructionContent>,
        // sub: Option<BannerInstructionContent>,
    }

    impl BannerInstruction {
        fn from_maneuver(value: &Maneuver) -> Option<Vec<Self>> {
            let text = value.street_names.as_ref().map(|names| names.join(", "));

            let banner_maneuver = (|| {
                use BannerManeuverModifier::*;
                use BannerManeuverType::*;
                let (banner_type, modifier) = match value.r#type {
                    ManeuverType::None => return None,
                    ManeuverType::Start => (Depart, None),
                    ManeuverType::StartRight => (Depart, Some(Right)),
                    ManeuverType::StartLeft => (Depart, Some(Left)),
                    ManeuverType::Destination => (Arrive, None),
                    ManeuverType::DestinationRight => (Arrive, Some(Right)),
                    ManeuverType::DestinationLeft => (Arrive, Some(Left)),
                    /*
                    ManeuverType::Becomes => {}
                    */
                    ManeuverType::Continue => (Fork, None), // Or maybe just return None?
                    ManeuverType::SlightRight => (Turn, Some(SlightRight)),
                    ManeuverType::Right => (Turn, Some(Right)),
                    ManeuverType::SharpRight => (Turn, Some(SharpRight)),
                    /*
                    ManeuverType::UturnRight => {}
                    ManeuverType::UturnLeft => {}
                    */
                    ManeuverType::SharpLeft => (Turn, Some(SharpLeft)),
                    ManeuverType::Left => (Turn, Some(Left)),
                    ManeuverType::SlightLeft => (Turn, Some(SlightLeft)),
                    // REVIEW: Is OffRamp correct here?
                    ManeuverType::RampStraight => (OffRamp, Some(Straight)),
                    ManeuverType::RampRight => (OffRamp, Some(Right)),
                    ManeuverType::RampLeft => (OffRamp, Some(Left)),
                    ManeuverType::ExitRight => (OffRamp, Some(Right)),
                    ManeuverType::ExitLeft => (OffRamp, Some(Left)),
                    ManeuverType::StayStraight => (Fork, None), // Or maybe just return None?
                    ManeuverType::StayRight => (Fork, Some(Right)),
                    ManeuverType::StayLeft => (Fork, Some(Left)),
                    /*
                    ManeuverType::Merge => {}
                    ManeuverType::RoundaboutEnter => {}
                    ManeuverType::RoundaboutExit => {}
                    ManeuverType::FerryEnter => {}
                    ManeuverType::FerryExit => {}
                    ManeuverType::Transit => {}
                    ManeuverType::TransitTransfer => {}
                    ManeuverType::TransitRemainOn => {}
                    ManeuverType::TransitConnectionStart => {}
                    ManeuverType::TransitConnectionTransfer => {}
                    ManeuverType::TransitConnectionDestination => {}
                    ManeuverType::PostTransitConnectionDestination => {}
                    ManeuverType::MergeRight => {}
                    ManeuverType::MergeLeft => {}
                    ManeuverType::ElevatorEnter => {}
                    ManeuverType::StepsEnter => {}
                    ManeuverType::EscalatorEnter => {}
                    ManeuverType::BuildingEnter => {}
                    ManeuverType::BuildingExit => {}
                     */
                    _ => todo!("implement manuever type: {:?}", value.r#type),
                };
                Some(BannerManeuver {
                    r#type: banner_type,
                    modifier,
                })
            })();

            let text_component = BannerComponent::Text(VisualInstructionComponent { text });
            //     if let Some(banner_maneuver) = banner_maneuver {
            //         BannerComponent::Text(VisualInstructionComponent {
            //             text,
            //         })
            //     } else {
            //         panic!("no banner_maneuver")
            //     }
            // };

            let primary = BannerInstructionContent {
                text: value.instruction.clone()?,
                components: vec![text_component], // TODO
                banner_maneuver,
                degrees: None,
                driving_side: None,
            };
            let instruction = BannerInstruction {
                distance_along_geometry: 0.0,
                primary,
            };
            Some(vec![instruction])
        }
    }

    // REVIEW: Rename to VisualInstructionBanner?
    // How do audible instructions fit into this?
    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    struct BannerInstructionContent {
        text: String,
        // components: Vec<BannerInstructionContentComponent>,
        #[serde(flatten)]
        banner_maneuver: Option<BannerManeuver>,
        degrees: Option<f64>,
        driving_side: Option<String>,
        components: Vec<BannerComponent>,
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "lowercase")]
    struct BannerManeuver {
        r#type: BannerManeuverType,
        modifier: Option<BannerManeuverModifier>,
    }

    // This is for `banner.primary(et. al).type`
    // There is a lot of overlap between this and `step_maneuver.type`,
    // but the docs imply they are different.
    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "lowercase")]
    enum BannerManeuverType {
        Turn,
        Merge,
        Depart,
        Arrive,
        Fork,
        #[serde(rename = "off ramp")]
        OffRamp,
        RoundAbout,
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "lowercase")]
    enum BannerManeuverModifier {
        Uturn,
        #[serde(rename = "sharp right")]
        SharpRight,
        Right,
        #[serde(rename = "slight right")]
        SlightRight,
        Straight,
        #[serde(rename = "slight left")]
        SlightLeft,
        Left,
        #[serde(rename = "sharp left")]
        SharpLeft,
    }

    // REVIEW: Rename to VisualInstruction?
    // REVIEW: convert to inner enum of Lane or VisualInstructionComponent
    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase", tag = "type")]
    #[non_exhaustive]
    enum BannerComponent {
        Text(VisualInstructionComponent),
        // Icon(VisualInstructionComponent),
        // Delimiter(VisualInstructionComponent),
        // #[serde(rename="exit-number")]
        // ExitNumber(VisualInstructionComponent),
        // Exit(VisualInstructionComponent),
        Lane(LaneInstructionComponent),
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    struct LaneInstructionComponent {}

    // Maybe we won't use this? Because it'll need to be implicit in the containing BannerComponent enum variant of
    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "lowercase")]
    enum VisualInstructionComponentType {
        /// The component separates two other destination components.
        ///
        /// If the two adjacent components are both displayed as images, you can hide this delimiter component.
        Delimiter,

        /// The component bears the name of a place or street.
        Text,

        /// Component contains an image that should be rendered.
        Image,

        /// The component contains the localized word for "exit".
        ///
        /// This component may appear before or after an `.ExitCode` component, depending on the language.
        Exit,

        /// A component contains an exit number.
        #[serde(rename = "exit-number")]
        ExitCode,
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    struct VisualInstructionComponent {
        text: Option<String>,
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StepManeuver {
        /// The location of the maneuver
        #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
        pub location: Point,
        // /// The type of maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
        // r#type: String,
        // /// The modifier of the maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
        // modifier: String,
        // /// A human-readable instruction of how to execute the returned maneuver
        // instruction: String,
        // /// The bearing before the turn, in degrees
        // OSRM expects `bearing_before` to be snake cased.
        // #[serde(rename_all = "snake_case")]
        // bearing_before: f64,
        // OSRM expects `bearing_after` to be snake cased.
        // /// The bearing after the turn, in degrees
        // bearing_after: f64,
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Intersection {
        /// A [longitude, latitude] pair describing the location of the turn.
        #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
        pub location: Point,

        /// A list of bearing values that are available at the intersection. The bearings describe all available roads at the intersection.
        pub bearings: Vec<f64>,

        // /// An array of strings signifying the classes of the road exiting the intersection.
        // pub classes: Vec<RoadClass>
        /// A list of entry flags, corresponding in a 1:1 relationship to the bearings. A value of true indicates that the respective road could be entered on a valid route.
        pub entry: Vec<bool>,

        /// The zero-based index into the geometry, relative to the start of the leg it's on. This value can be used to apply the duration annotation that corresponds with the intersection.
        pub geometry_index: Option<usize>,

        /// The index in the bearings and entry arrays. Used to calculate the bearing before the turn. Namely, the clockwise angle from true north to the direction of travel before the maneuver/passing the intersection. To get the bearing in the direction of driving, the bearing has to be rotated by a value of 180. The value is not supplied for departure maneuvers.
        pub r#in: Option<usize>,

        /// The index in the bearings and entry arrays. Used to extract the bearing after the turn. Namely, the clockwise angle from true north to the direction of travel after the maneuver/passing the intersection. The value is not supplied for arrival maneuvers.
        pub out: Option<usize>,

        /// An array of lane objects that represent the available turn lanes at the intersection. If no lane information is available for an intersection, the lanes property will not be present.
        pub lanes: Vec<Lane>,

        /// The time required, in seconds, to traverse the intersection. Only available on the driving profile.
        pub duration: Option<f64>,
        // TODO: lots more fields in OSRM
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Lane {
        // TODO: lots more fields in OSRM
    }
}

impl From<PlanResponseOk> for DirectionsResponseOk {
    fn from(value: PlanResponseOk) -> Self {
        let routes = value
            .plan
            .itineraries
            .into_iter()
            .map(osrm_api::Route::from)
            .collect();
        Self { routes }
    }
}

#[get("/v5/plan")]
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

    let distance_units = query
        .preferred_distance_units
        .unwrap_or(DistanceUnit::Kilometers);

    // TODO: Handle bus+bike if bike is first, for now all our clients are responsible for enforcing that
    // the "primary" mode appears first.
    match primary_mode {
        TravelMode::Transit => otp_plan(&query, req, &app_state, primary_mode).await,
        other => {
            debug_assert!(query.mode.len() == 1, "valhalla only supports one mode");

            let mode = match other {
                TravelMode::Transit => unreachable!("handled above"),
                TravelMode::Bicycle => valhalla_api::ModeCosting::Bicycle,
                TravelMode::Car => valhalla_api::ModeCosting::Auto,
                TravelMode::Walk => valhalla_api::ModeCosting::Pedestrian,
            };

            // route?json={%22locations%22:[{%22lat%22:47.575837,%22lon%22:-122.339414},{%22lat%22:47.651048,%22lon%22:-122.347234}],%22costing%22:%22auto%22,%22alternates%22:3,%22units%22:%22miles%22}
            let router_url = app_state.valhalla_router().plan_url(
                query.from_place,
                query.to_place,
                mode,
                query.num_itineraries,
                distance_units,
            )?;
            let valhalla_response: reqwest::Response =
                reqwest::get(router_url).await.map_err(|e| {
                    log::error!("error while fetching from valhalla service: {e}");
                    PlanResponseErr::from(Error::server(e))
                })?;
            if !valhalla_response.status().is_success() {
                log::warn!(
                    "upstream HTTP Error from valhalla service: {}",
                    valhalla_response.status()
                )
            }

            let mut response = HttpResponseBuilder::new(valhalla_response.status());
            debug_assert_eq!(
                valhalla_response
                    .headers()
                    .get(HeaderName::from_static("content-type")),
                Some(&HeaderValue::from_str("application/json;charset=utf-8").unwrap())
            );
            response.content_type("application/json;charset=utf-8");

            let valhalla_route_response: valhalla_api::ValhallaRouteResponseResult =
                valhalla_response.json().await.map_err(|e| {
                    log::error!("error while parsing valhalla response: {e}");
                    PlanResponseErr::from(Error::server(e))
                })?;

            let mut plan_response =
                PlanResponseOk::from_valhalla(*primary_mode, valhalla_route_response)?;

            if primary_mode == &TravelMode::Bicycle || primary_mode == &TravelMode::Walk {
                match otp_plan(&query, req, &app_state, primary_mode).await {
                    Ok(mut otp_response) => {
                        debug_assert_eq!(
                            1,
                            otp_response.plan.itineraries.len(),
                            "expected exactly one itinerary from OTP"
                        );
                        if let Some(otp_itinerary) = otp_response.plan.itineraries.pop() {
                            log::debug!("adding OTP itinerary to valhalla response");
                            plan_response.plan.itineraries.insert(0, otp_itinerary);
                        }
                    }
                    Err(e) => {
                        // match error_code to raw value of ErrorType enum
                        match ErrorType::try_from(e.error.error_code) {
                            Ok(ErrorType::NoCoverageForArea) => {
                                log::debug!("No OTP coverage for route");
                            }
                            other => {
                                debug_assert!(other.is_ok(), "unexpected error code: {e:?}");
                                // We're mixing with results from Valhalla anyway, so dont' surface this error
                                // to the user. Likely we just don't support this area.
                                log::error!("OTP failed to plan {primary_mode:?} route: {e}");
                            }
                        }
                    }
                }
            }

            Ok(plan_response)
        }
    }
}

async fn otp_plan(
    query: &web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: &web::Data<AppState>,
    primary_mode: &TravelMode,
) -> Result<PlanResponseOk, PlanResponseErr> {
    let Some(mut router_url) = app_state
        .otp_cluster()
        .find_router_url(query.from_place, query.to_place)
    else {
        Err(
            Error::user("Transit directions not available for this area.")
                .error_type(ErrorType::NoCoverageForArea),
        )?
    };

    // if we end up building this manually rather than passing it through, we'll need to be sure
    // to handle the bike+bus case
    router_url.set_query(Some(req.query_string()));
    log::debug!(
        "found matching router. Forwarding request to: {}",
        router_url
    );

    let otp_response: reqwest::Response = reqwest::get(router_url).await.map_err(|e| {
        log::error!("error while fetching from otp service: {e}");
        PlanResponseErr::from(Error::server(e))
    })?;
    if !otp_response.status().is_success() {
        log::warn!(
            "upstream HTTP Error from otp service: {}",
            otp_response.status()
        )
    }

    let mut response = HttpResponseBuilder::new(otp_response.status());
    debug_assert_eq!(
        otp_response
            .headers()
            .get(HeaderName::from_static("content-type")),
        Some(&HeaderValue::from_str("application/json").unwrap())
    );
    response.content_type("application/json");

    let otp_plan_response: otp_api::PlanResponse = otp_response.json().await.map_err(|e| {
        log::error!("error while parsing otp response: {e}");
        PlanResponseErr::from(Error::server(e))
    })?;
    PlanResponseOk::from_otp(
        *primary_mode,
        otp_plan_response,
        query
            .preferred_distance_units
            .unwrap_or(DistanceUnit::Kilometers),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use serde_json::{json, Value};
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn parse_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_route_walk.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let valhalla_response_result = valhalla_api::ValhallaRouteResponseResult::Ok(valhalla);
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response_result).unwrap();
        assert_eq!(plan_response.plan.itineraries.len(), 3);

        // itineraries
        let first_itinerary = &plan_response.plan.itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Walk);
        assert_relative_eq!(first_itinerary.distance, 9.148);
        assert_relative_eq!(first_itinerary.duration, 6488.443);
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

    #[test]
    fn parse_from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_plan_transit.json").unwrap();
        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response =
            PlanResponseOk::from_otp(TravelMode::Transit, otp, DistanceUnit::Miles).unwrap();

        let itineraries = plan_response.plan.itineraries;
        assert_eq!(itineraries.len(), 5);

        // itineraries
        let first_itinerary = &itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Transit);
        assert_relative_eq!(first_itinerary.distance, 10.69944);
        assert_relative_eq!(first_itinerary.duration, 3273.0);

        // legs
        assert_eq!(first_itinerary.legs.len(), 7);
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
        assert_eq!(maneuvers.len(), 4);
        assert_eq!(maneuvers[0].r#type, ManeuverType::Start);
        assert_eq!(maneuvers[1].r#type, ManeuverType::Left);

        let fourth_leg = &first_itinerary.legs[3];
        assert_eq!(fourth_leg.mode, TravelMode::Transit);
        let ModeLeg::Transit(transit_leg) = &fourth_leg.mode_leg else {
            panic!("expected transit leg")
        };
        assert_eq!(transit_leg.route_color, Some("28813F".to_string()));
    }

    #[test]
    fn serialize_response_from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_plan_transit.json").unwrap();
        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response =
            PlanResponseOk::from_otp(TravelMode::Transit, otp, DistanceUnit::Miles).unwrap();
        let response = serde_json::to_string(&plan_response).unwrap();
        let parsed_response: serde_json::Value = serde_json::from_str(&response).unwrap();
        let response_object = parsed_response.as_object().expect("expected Object");
        let plan = response_object
            .get("plan")
            .unwrap()
            .as_object()
            .expect("expected Object");
        let first_itinerary = plan
            .get("itineraries")
            .unwrap()
            .as_array()
            .unwrap()
            .get(0)
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.get(0).unwrap().as_object().unwrap();
        let mode = first_leg.get("mode").unwrap().as_str().unwrap();
        assert_eq!(mode, "WALK");

        let mode = first_leg
            .get("startTime")
            .expect("field missing")
            .as_u64()
            .expect("unexpected type. expected u64");
        assert_eq!(mode, 1708728373000);

        let mode = first_leg
            .get("endTime")
            .expect("field missing")
            .as_u64()
            .expect("unexpected type. expected u64");
        assert_eq!(mode, 1708728745000);

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
        let first_maneuver = maneuvers.get(0).unwrap();
        let expected_maneuver = json!({
            "distance": 11.893,
            "instruction": "Walk south on East Marginal Way South.",
            "startPoint": {
                "lat": 47.5758355,
                "lon": -122.3392164
            },
            "streetNames": ["East Marginal Way South"],
            "type": 1,
            "verbalPostTransitionInstruction": "Continue for 60 feet."
        });
        assert_eq!(first_maneuver, &expected_maneuver);

        // Verify Transit leg
        let fourth_leg = legs.get(3).unwrap().as_object().unwrap();
        let mode = fourth_leg.get("mode").unwrap().as_str().unwrap();
        assert_eq!(mode, "TRANSIT");
        assert!(fourth_leg.get("maneuvers").is_none());
        let transit_leg = fourth_leg
            .get("transitLeg")
            .unwrap()
            .as_object()
            .expect("json object");

        // Brittle: If the fixtures are updated, these values might change due to time of day or whatever.
        assert_eq!(
            transit_leg.get("agencyName").unwrap().as_str().unwrap(),
            "Sound Transit"
        );

        assert_eq!(
            transit_leg.get("route").unwrap().as_str().unwrap(),
            "Northgate - Angle Lake"
        );
    }

    #[test]
    fn serialize_response_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_route_walk.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let valhalla_response_result = valhalla_api::ValhallaRouteResponseResult::Ok(valhalla);
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response_result).unwrap();

        let response = serde_json::to_string(&plan_response).unwrap();
        let parsed_response: serde_json::Value = serde_json::from_str(&response).unwrap();
        let response_object = parsed_response.as_object().expect("expected Object");
        let plan = response_object
            .get("plan")
            .unwrap()
            .as_object()
            .expect("expected Object");
        let first_itinerary = plan
            .get("itineraries")
            .unwrap()
            .as_array()
            .unwrap()
            .get(0)
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.get(0).unwrap().as_object().unwrap();
        let mode = first_leg.get("mode").unwrap().as_str().unwrap();
        assert_eq!(mode, "WALK");
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
        let first_maneuver = maneuvers.get(0).unwrap();
        let expected_maneuver = json!({
            "type": 2,
            "instruction": "Walk south on East Marginal Way South.",
            "verbalPostTransitionInstruction": "Continue for 20 meters.",
            "distance": 0.019,
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
        let geometry = LineString::from(vec![
            geo::coord!(x: -122.398, y: 47.564),
            geo::coord!(x: -122.396, y: 47.566),
        ]);
        let maneuver = Maneuver::from_valhalla(valhalla_maneuver, &geometry);
        let actual = serde_json::to_string(&maneuver).unwrap();
        // parse the JSON string back into an Object Value
        let actual_object: Value = serde_json::from_str(&actual).unwrap();

        let expected_object = serde_json::json!({
            "instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "type": 2,
            "distance": 2.218,
            "startPoint": { "lon": -122.398, "lat": 47.564},
            "streetNames": ["Fauntleroy Way Southwest"],
            "verbalPostTransitionInstruction": "Continue for 2 miles.",
        });

        assert_eq!(actual_object, expected_object);
    }

    #[test]
    fn parse_error_from_valhalla() {
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
    fn navigation_response_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_route_walk.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let valhalla_response_result = valhalla_api::ValhallaRouteResponseResult::Ok(valhalla);
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response_result).unwrap();

        let directions_response = DirectionsResponseOk::from(plan_response);
        assert_eq!(directions_response.routes.len(), 3);

        let first_route = &directions_response.routes[0];
        assert_relative_eq!(first_route.distance, 9.148);
        assert_relative_eq!(first_route.duration, 6488.443);
        assert_relative_eq!(
            first_route.geometry.0[0],
            geo::coord!(x: -122.339216, y: 47.575836)
        );
        assert_relative_eq!(
            first_route.geometry.0.last().unwrap(),
            &geo::coord!(x: -122.347199, y: 47.651048)
        );

        let legs = &first_route.legs;
        assert_eq!(legs.len(), 1);
        let first_leg = &legs[0];

        assert_eq!(first_leg.distance, 9.148);
        assert_eq!(first_leg.duration, 6488.443);
        assert_eq!(
            first_leg.summary,
            "Dexter Avenue, East Marginal Way South, Alaskan Way South"
        );
        assert_eq!(first_leg.steps.len(), 21);

        let first_step = &first_leg.steps[0];
        assert_eq!(first_step.distance, 0.019);
        assert_eq!(first_step.duration, 13.567);
        assert_eq!(first_step.name, "East Marginal Way South");
        assert_eq!(first_step.mode, TravelMode::Walk);

        let step_maneuver = &first_step.maneuver;
        assert_eq!(
            step_maneuver.location,
            geo::point!(x: -122.339216, y: 47.575836)
        );
        // TODO: step_maneuver stuff
        // etc...
    }
}
