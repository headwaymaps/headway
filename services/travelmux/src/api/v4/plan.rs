use actix_web::{get, web, HttpRequest, HttpResponseBuilder};
use geo::algorithm::BoundingRect;
use geo::geometry::{LineString, Point, Rect};
use polyline::decode_polyline;
use reqwest::header::{HeaderName, HeaderValue};
use serde::{de, de::IntoDeserializer, de::Visitor, Deserialize, Deserializer, Serialize};
use std::fmt;

use super::error::{PlanResponseErr, PlanResponseOk};
use crate::api::AppState;
use crate::error::ErrorType;
use crate::otp::otp_api;
use crate::otp::otp_api::RelativeDirection;
use crate::util::serialize_rect_to_lng_lat;
use crate::util::{deserialize_point_from_lat_lon, extend_bounds};
use crate::valhalla::valhalla_api;
use crate::valhalla::valhalla_api::ManeuverType;
use crate::{DistanceUnit, Error, TravelMode};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub(crate) itineraries: Vec<Itinerary>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    mode: TravelMode,
    /// seconds
    duration: f64,
    /// unix millis, UTC
    start_time: u64,
    /// unix millis, UTC
    end_time: u64,
    distance: f64,
    distance_units: DistanceUnit,
    #[serde(serialize_with = "serialize_rect_to_lng_lat")]
    bounds: Rect,
    legs: Vec<Leg>,
}

impl Itinerary {
    pub fn from_valhalla(valhalla: &valhalla_api::Trip, mode: TravelMode) -> Self {
        let bounds = Rect::new(
            geo::coord!(x: valhalla.summary.min_lon, y: valhalla.summary.min_lat),
            geo::coord!(x: valhalla.summary.max_lon, y: valhalla.summary.max_lat),
        );

        use std::time::Duration;
        fn time_since_epoch() -> Duration {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after unix epoch")
        }

        let start_time = time_since_epoch().as_millis() as u64;
        let end_time = start_time + (valhalla.summary.time * 1000.0) as u64;
        Self {
            mode,
            start_time,
            end_time,
            duration: valhalla.summary.time,
            distance: valhalla.summary.length,
            bounds,
            distance_units: valhalla.units,
            legs: valhalla
                .legs
                .iter()
                .map(|v_leg| Leg::from_valhalla(v_leg, mode))
                .collect(),
        }
    }

    pub fn from_otp(itinerary: &otp_api::Itinerary, mode: TravelMode) -> crate::Result<Self> {
        // OTP responses are always in meters
        let distance_meters: f64 = itinerary.legs.iter().map(|l| l.distance).sum();
        let Ok(legs): std::result::Result<Vec<_>, _> =
            itinerary.legs.iter().map(Leg::from_otp).collect()
        else {
            return Err(Error::server("failed to parse legs"));
        };

        let mut legs_iter = legs.iter();
        let Some(first_leg) = legs_iter.next() else {
            return Err(Error::server("itinerary had no legs"));
        };
        let Ok(Some(mut itinerary_bounds)) = first_leg.bounding_rect() else {
            return Err(Error::server("first leg has no bounding_rect"));
        };
        for leg in legs_iter {
            let Ok(Some(leg_bounds)) = leg.bounding_rect() else {
                return Err(Error::server("leg has no bounding_rect"));
            };
            extend_bounds(&mut itinerary_bounds, &leg_bounds);
        }
        Ok(Self {
            duration: itinerary.duration as f64,
            start_time: itinerary.start_time,
            end_time: itinerary.end_time,
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
struct Leg {
    /// encoded polyline. 1e-6 scale, (lat, lon)
    geometry: String,

    /// Which mode is this leg of the journey?
    mode: TravelMode,

    #[serde(flatten)]
    mode_leg: ModeLeg,
}

// Should we just pass the entire OTP leg?
type TransitLeg = otp_api::Leg;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ModeLeg {
    // REVIEW: rename? There is a boolean field for OTP called TransitLeg
    #[serde(rename = "transitLeg")]
    Transit(TransitLeg),

    #[serde(rename = "maneuvers")]
    NonTransit(Vec<Maneuver>),
}

// Eventually we might want to coalesce this into something not valhalla specific
// but for now we only use it for valhalla trips
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Maneuver {
    pub instruction: Option<String>,
    // pub cost: f64,
    // pub begin_shape_index: u64,
    // pub end_shape_index: u64,
    // pub highway: Option<bool>,
    // pub length: f64,
    // pub street_names: Option<Vec<String>>,
    // pub time: f64,
    // pub travel_mode: String,
    // pub travel_type: String,
    pub r#type: ManeuverType,
    pub verbal_post_transition_instruction: Option<String>,
    // pub verbal_pre_transition_instruction: Option<String>,
    // pub verbal_succinct_transition_instruction: Option<String>,
}

impl Maneuver {
    fn from_valhalla(valhalla: valhalla_api::Maneuver) -> Self {
        Self {
            instruction: Some(valhalla.instruction),
            r#type: valhalla.r#type,
            verbal_post_transition_instruction: valhalla.verbal_post_transition_instruction,
        }
    }

    fn from_otp(otp: otp_api::Step) -> Self {
        let maneuver_type = match otp.relative_direction {
            RelativeDirection::Depart => ManeuverType::Start,
            RelativeDirection::HardLeft => ManeuverType::SharpLeft,
            RelativeDirection::Left => ManeuverType::Left,
            RelativeDirection::SlightlyLeft => ManeuverType::SlightLeft,
            RelativeDirection::Continue => ManeuverType::Continue,
            RelativeDirection::SlightlyRight => ManeuverType::SlightRight,
            RelativeDirection::Right => ManeuverType::Right,
            RelativeDirection::HardRight => ManeuverType::SharpRight,
            RelativeDirection::CircleClockwise => ManeuverType::RoundaboutEnter,
            RelativeDirection::CircleCounterclockwise => ManeuverType::RoundaboutEnter,
            RelativeDirection::Elevator => ManeuverType::ElevatorEnter,
            RelativeDirection::UturnLeft => ManeuverType::UturnLeft,
            RelativeDirection::UturnRight => ManeuverType::UturnRight,
        };
        Self {
            instruction: None,
            r#type: maneuver_type,
            verbal_post_transition_instruction: None,
        }
    }
}

impl Leg {
    const GEOMETRY_PRECISION: u32 = 6;

    fn decoded_geometry(&self) -> std::result::Result<LineString, String> {
        decode_polyline(&self.geometry, Self::GEOMETRY_PRECISION)
    }

    fn bounding_rect(&self) -> std::result::Result<Option<Rect>, String> {
        let line_string = self.decoded_geometry()?;
        Ok(line_string.bounding_rect())
    }

    fn from_otp(otp: &otp_api::Leg) -> std::result::Result<Self, String> {
        let line = decode_polyline(&otp.leg_geometry.points, 5)?;
        let geometry = polyline::encode_coordinates(line, Self::GEOMETRY_PRECISION)?;

        let mode_leg = match otp.mode {
            otp_api::TransitMode::Walk
            | otp_api::TransitMode::Bicycle
            | otp_api::TransitMode::Car => {
                ModeLeg::NonTransit(otp.steps.iter().cloned().map(Maneuver::from_otp).collect())
            }
            _ => {
                // assume everything else is transit
                ModeLeg::Transit(otp.clone())
            }
        };

        Ok(Self {
            geometry,
            mode: otp.mode.into(),
            mode_leg,
        })
    }

    fn from_valhalla(valhalla: &valhalla_api::Leg, travel_mode: TravelMode) -> Self {
        Self {
            geometry: valhalla.shape.clone(),
            mode: travel_mode,
            mode_leg: ModeLeg::NonTransit(
                valhalla
                    .maneuvers
                    .iter()
                    .cloned()
                    .map(Maneuver::from_valhalla)
                    .collect(),
            ),
        }
    }
}

// Comma separated list of travel modes
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
struct TravelModes(Vec<TravelMode>);

impl<'de> Deserialize<'de> for TravelModes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CommaSeparatedVecVisitor;

        impl<'de> Visitor<'de> for CommaSeparatedVecVisitor {
            type Value = TravelModes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a comma-separated string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                let modes = value
                    .split(',')
                    .map(|s| TravelMode::deserialize(s.into_deserializer()))
                    .collect::<std::result::Result<_, _>>()?;
                Ok(TravelModes(modes))
            }
        }

        deserializer.deserialize_str(CommaSeparatedVecVisitor)
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

#[get("/v4/plan")]
pub async fn get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> std::result::Result<PlanResponseOk, PlanResponseErr> {
    let Some(primary_mode) = query.mode.0.first() else {
        return Err(PlanResponseErr::from(Error::user("mode is required")));
    };

    let distance_units = query
        .preferred_distance_units
        .unwrap_or(DistanceUnit::Kilometers);

    // TODO: Handle bus+bike if bike is first, for now all our clients are responsible for enforcing that
    // the "primary" mode appears first.
    match primary_mode {
        TravelMode::Transit => {
            let Some(mut router_url) = app_state
                .otp_cluster()
                .find_router_url(query.from_place, query.to_place)
            else {
                Err(
                    Error::user("Transit directions not available for this area.")
                        .error_type(ErrorType::ThisTransitAreaNotCovered),
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

            let otp_plan_response: otp_api::PlanResponse =
                otp_response.json().await.map_err(|e| {
                    log::error!("error while parsing otp response: {e}");
                    PlanResponseErr::from(Error::server(e))
                })?;

            let plan_response = PlanResponseOk::from_otp(*primary_mode, otp_plan_response)?;
            Ok(plan_response)
        }
        other => {
            debug_assert!(query.mode.0.len() == 1, "valhalla only supports one mode");

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

            let plan_response =
                PlanResponseOk::from_valhalla(*primary_mode, valhalla_route_response)?;
            Ok(plan_response)
        }
    }
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
        let geometry = decode_polyline(&first_leg.geometry, 6).unwrap();
        assert_relative_eq!(
            geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583),
            epsilon = 1e-4
        );

        let ModeLeg::NonTransit(maneuvers) = &first_leg.mode_leg else {
            panic!("unexpected transit leg")
        };

        assert_eq!(first_leg.mode, TravelMode::Walk);
        assert_eq!(maneuvers.len(), 21);
    }

    #[test]
    fn parse_from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_plan_transit.json").unwrap();
        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response = PlanResponseOk::from_otp(TravelMode::Transit, otp).unwrap();

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
        let geometry = polyline::decode_polyline(&first_leg.geometry, 6).unwrap();
        assert_relative_eq!(
            geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583),
            epsilon = 1e-4
        );

        assert_eq!(first_leg.mode, TravelMode::Walk);
        let ModeLeg::NonTransit(maneuvers) = &first_leg.mode_leg else {
            panic!("expected non-transit leg")
        };
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
        let plan_response = PlanResponseOk::from_otp(TravelMode::Transit, otp).unwrap();
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
        let maneuvers = first_leg.get("maneuvers").unwrap().as_array().unwrap();
        let first_maneuver = maneuvers.get(0).unwrap();
        let expected_maneuver = json!({
            "type": 1,
            "instruction": null,
            "verbalPostTransitionInstruction": null
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
        dbg!(transit_leg);

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
        let maneuvers = first_leg.get("maneuvers").unwrap().as_array().unwrap();
        let first_maneuver = maneuvers.get(0).unwrap();
        let expected_maneuver = json!({
            "type": 2,
            "instruction": "Walk south on East Marginal Way South.",
            "verbalPostTransitionInstruction": "Continue for 20 meters."
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
            "end_shape_index": 69,
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

        let maneuver = Maneuver::from_valhalla(valhalla_maneuver);
        let actual = serde_json::to_string(&maneuver).unwrap();
        // parse the JSON string back into an Object Value
        let actual_object: Value = serde_json::from_str(&actual).unwrap();

        let expected_object = serde_json::json!({
            "instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "type": 2,
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
}