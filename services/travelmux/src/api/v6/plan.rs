use actix_web::web::{Data, Query};
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
use crate::util::format::format_meters;
use crate::util::haversine_segmenter::HaversineSegmenter;
use crate::util::serde_util::{
    deserialize_point_from_lat_lon, serialize_line_string_as_polyline6, serialize_rect_to_lng_lat,
    serialize_system_time_as_millis,
};
use crate::util::{convert_from_meters, convert_to_meters, extend_bounds, system_time_from_millis};
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
    pub(crate) duration: f64,
    /// unix millis, UTC
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    start_time: SystemTime,
    /// unix millis, UTC
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    end_time: SystemTime,
    /// Units are in `distance_units`
    distance: f64,
    pub(crate) distance_units: DistanceUnit,
    #[serde(serialize_with = "serialize_rect_to_lng_lat")]
    bounds: Rect,
    pub(crate) legs: Vec<Leg>,
}

impl Itinerary {
    pub fn distance_meters(&self) -> f64 {
        convert_to_meters(self.distance, self.distance_units)
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
            distance: convert_from_meters(distance_meters, distance_unit),
            distance_units: distance_unit,
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

    /// End of the Leg
    to_place: Place,

    /// Start time of the leg
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    start_time: SystemTime,

    /// Start time of the leg
    #[serde(serialize_with = "serialize_system_time_as_millis")]
    end_time: SystemTime,

    /// Length of this leg, in units of the `distance_unit` of the Itinerary
    distance: f64,

    /// Duration of this leg
    pub(crate) duration_seconds: f64,
}

// Currently we just pass the entire OTP leg
type TransitLeg = otp_api::Leg;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ModeLeg {
    #[serde(rename = "transitLeg")]
    Transit(Box<TransitLeg>),

    #[serde(rename = "nonTransitLeg")]
    NonTransit(Box<NonTransitLeg>),
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

/// One action taken by the user - like a turn or taking an exit.
/// This was originally based on the schema of a valhall_api::Maneuver, but it can be built from
/// either OTP or Valhalla data.
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
        geometry: LineString,
        leg: &otp_api::Leg,
        distance_unit: DistanceUnit,
    ) -> Self {
        let instruction = maneuver_instruction(
            leg.mode,
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

        let duration_seconds = otp.distance / leg.distance * leg.duration_seconds();
        Self {
            instruction,
            r#type: otp.relative_direction.into(),
            street_names,
            verbal_post_transition_instruction,
            distance: convert_from_meters(otp.distance, distance_unit),
            duration_seconds,
            start_point: Point::new(otp.lon, otp.lat).into(),
            geometry,
        }
    }

    pub(crate) fn distance_meters(&self, distance_unit: DistanceUnit) -> f64 {
        convert_to_meters(self.distance, distance_unit)
    }
}

/// Returns the natural language description of the maneuver.
// We could do so much better. Look at Valhalla's Odin.
//
// e.g. take context of previous maneuver. "Bear right to stay on Main Street"
// TODO: localize
fn maneuver_instruction(
    mode: otp_api::TransitMode,
    maneuver_type: otp_api::RelativeDirection,
    absolute_direction: Option<otp_api::AbsoluteDirection>,
    street_name: &str,
) -> Option<String> {
    match maneuver_type {
        otp_api::RelativeDirection::Depart => {
            if let Some(absolute_direction) = absolute_direction {
                let direction = match absolute_direction {
                    otp_api::AbsoluteDirection::North => "north",
                    otp_api::AbsoluteDirection::Northeast => "northeast",
                    otp_api::AbsoluteDirection::East => "east",
                    otp_api::AbsoluteDirection::Southeast => "southeast",
                    otp_api::AbsoluteDirection::South => "south",
                    otp_api::AbsoluteDirection::Southwest => "southwest",
                    otp_api::AbsoluteDirection::West => "west",
                    otp_api::AbsoluteDirection::Northwest => "northwest",
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
        otp_api::RelativeDirection::HardLeft => {
            Some(format!("Turn sharp left onto {street_name}."))
        }
        otp_api::RelativeDirection::Left => Some(format!("Turn left onto {street_name}.")),
        otp_api::RelativeDirection::SlightlyLeft => {
            Some(format!("Turn slightly left onto {street_name}."))
        }
        otp_api::RelativeDirection::Continue => Some(format!("Continue onto {street_name}.")),
        otp_api::RelativeDirection::SlightlyRight => {
            Some(format!("Turn slightly right onto {street_name}."))
        }
        otp_api::RelativeDirection::Right => Some(format!("Turn right onto {street_name}.")),
        otp_api::RelativeDirection::HardRight => {
            Some(format!("Turn sharp right onto {street_name}."))
        }
        otp_api::RelativeDirection::CircleClockwise
        | otp_api::RelativeDirection::CircleCounterclockwise => {
            Some("Enter the roundabout.".to_string())
        }
        otp_api::RelativeDirection::Elevator => Some("Enter the elevator.".to_string()),
        otp_api::RelativeDirection::UturnLeft | otp_api::RelativeDirection::UturnRight => {
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
            format_meters(distance, distance_unit.measurement_system())
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

    pub(crate) fn distance_meters(&self, itinerary_units: DistanceUnit) -> f64 {
        convert_to_meters(self.distance, itinerary_units)
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

        let mut distance_so_far = 0.0;
        let mut segmenter = HaversineSegmenter::new(geometry.clone());
        let mode_leg = match otp.mode {
            otp_api::TransitMode::Walk
            | otp_api::TransitMode::Bicycle
            | otp_api::TransitMode::Car => {
                let mut maneuvers: Vec<_> = otp
                    .steps
                    .iter()
                    .cloned()
                    .map(|otp_step| {
                        // compute step geometry by distance along leg geometry
                        let step_geometry = segmenter
                            .next_segment(otp_step.distance)
                            .unwrap_or_else(|| {
                                log::warn!("no geometry for step");
                                debug_assert!(false, "no geometry for step");
                                LineString::new(vec![])
                            });
                        distance_so_far += otp_step.distance;
                        Maneuver::from_otp(otp_step, step_geometry, otp, distance_unit)
                    })
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
            distance: convert_from_meters(otp.distance, distance_unit),
            duration_seconds: otp.duration_seconds(),
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
            distance: valhalla.summary.length,
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

#[get("/v6/plan")]
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
            if primary_mode == &TravelMode::Bicycle || primary_mode == &TravelMode::Walk {
                match otp_plan(&query, req, &app_state, primary_mode).await {
                    Ok(otp_response) => {
                        debug_assert_eq!(
                            1,
                            otp_response.plan.itineraries.len(),
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
            Ok(valhalla_plan(&query, &app_state, primary_mode, distance_units, other).await?)
        }
    }
}

async fn valhalla_plan(
    query: &Query<PlanQuery>,
    app_state: &Data<AppState>,
    primary_mode: &TravelMode,
    distance_units: DistanceUnit,
    other: &TravelMode,
) -> Result<PlanResponseOk, PlanResponseErr> {
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

    Ok(PlanResponseOk::from_valhalla(
        *primary_mode,
        valhalla_route_response,
    )?)
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
    use crate::util::{bearing_at_end, bearing_at_start};
    use approx::assert_relative_eq;
    use geo::wkt;
    use serde_json::{json, Value};
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn parse_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_pedestrian_route.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let valhalla_response_result = valhalla_api::ValhallaRouteResponseResult::Ok(valhalla);
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response_result).unwrap();
        assert_eq!(plan_response.plan.itineraries.len(), 3);

        // itineraries
        let first_itinerary = &plan_response.plan.itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Walk);
        assert_relative_eq!(first_itinerary.distance, 5.684);
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
            File::open("tests/fixtures/requests/opentripplanner_transit_plan.json").unwrap();
        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response =
            PlanResponseOk::from_otp(TravelMode::Transit, otp, DistanceUnit::Miles).unwrap();

        let itineraries = plan_response.plan.itineraries;
        assert_eq!(itineraries.len(), 6);

        // itineraries
        let first_itinerary = &itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Transit);
        assert_relative_eq!(first_itinerary.distance, 6.311692992158277);
        assert_relative_eq!(first_itinerary.duration, 2347.0);

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

        let transit_leg = &first_itinerary.legs[2];
        assert_eq!(transit_leg.mode, TravelMode::Transit);
        let ModeLeg::Transit(transit_leg) = &transit_leg.mode_leg else {
            panic!("expected transit leg")
        };
        assert!(transit_leg.route_color.is_none());
    }

    #[test]
    fn serialize_response_from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_transit_plan.json").unwrap();
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
            .first()
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.first().unwrap().as_object().unwrap();
        let mode = first_leg.get("mode").unwrap().as_str().unwrap();
        assert_eq!(mode, "WALK");

        let mode = first_leg
            .get("startTime")
            .expect("field missing")
            .as_u64()
            .expect("unexpected type. expected u64");
        assert_eq!(mode, 1715974501000);

        let mode = first_leg
            .get("endTime")
            .expect("field missing")
            .as_u64()
            .expect("unexpected type. expected u64");
        assert_eq!(mode, 1715974870000);

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
            "distance": 0.0118992879068438, // TODO: truncate precision in serializer
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

        // Verify Transit leg
        let transit_leg = legs.get(2).unwrap().as_object().unwrap();
        let mode = transit_leg.get("mode").unwrap().as_str().unwrap();
        assert_eq!(mode, "TRANSIT");
        assert!(transit_leg.get("maneuvers").is_none());
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

        assert!(transit_leg.get("route").is_none());
    }

    #[test]
    fn serialize_response_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_pedestrian_route.json").unwrap();
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
            .first()
            .unwrap();
        let legs = first_itinerary.get("legs").unwrap().as_array().unwrap();

        // Verify walking leg
        let first_leg = legs.first().unwrap().as_object().unwrap();
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
        let first_maneuver = maneuvers.first().unwrap();
        let expected_maneuver = json!({
            "type": 2,
            "instruction": "Walk south on East Marginal Way South.",
            "verbalPostTransitionInstruction": "Continue for 60 feet.",
            "distance": 0.011,
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
    fn maneuver_bearing() {
        let a = wkt!(LINESTRING(0. 0.,1. 0.,1. 1.));
        let b = wkt!(LINESTRING(1. 1., 0. 1., 0. 0.));

        assert_eq!(90, bearing_at_start(&a).unwrap());
        assert_eq!(0, bearing_at_end(&a).unwrap());

        assert_eq!(270, bearing_at_start(&b).unwrap());
        assert_eq!(180, bearing_at_end(&b).unwrap());
    }
}
