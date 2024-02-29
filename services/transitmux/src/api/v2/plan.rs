use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use geo::geometry::Point;
use reqwest::header::{HeaderName, HeaderValue};
use serde::de::IntoDeserializer;
use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize};
use std::fmt;

use crate::api::AppState;
use crate::otp::otp_api;
use crate::valhalla::valhalla_api;
use crate::{util::deserialize_point_from_lat_lon, DistanceUnit, Error, TravelMode};

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
struct PlanResponse {
    // The raw response from the upstream OTP /plan service
    #[serde(rename = "_otp")]
    _otp: Option<otp_api::PlanResponse>,

    // The raw response from the upstream Valhalla /route service
    #[serde(rename = "_valhalla")]
    _valhalla: Option<valhalla_api::RouteResponse>,

    plan: Plan,
}

impl PlanResponse {
    fn from_otp(mode: TravelMode, otp: otp_api::PlanResponse) -> Self {
        let itineraries = otp
            .plan
            .itineraries
            .iter()
            .map(|itinerary: &otp_api::Itinerary| {
                // OTP responses are always in meters
                let distance_meters: f64 = itinerary.legs.iter().map(|l| l.distance).sum();
                Itinerary {
                    duration: itinerary.duration,
                    mode,
                    distance: distance_meters / 1000.0,
                    distance_units: DistanceUnit::Kilometers,
                    legs: itinerary.legs.iter().map(Leg::from_otp).collect(),
                }
            })
            .collect();

        PlanResponse {
            plan: Plan { itineraries },
            _otp: Some(otp),
            _valhalla: None,
        }
    }

    fn from_valhalla(mode: TravelMode, valhalla: valhalla_api::RouteResponse) -> Self {
        let mut itineraries = vec![Itinerary::from_valhalla_trip(&valhalla.trip, mode)];
        if let Some(alternates) = &valhalla.alternates {
            for alternate in alternates {
                itineraries.push(Itinerary::from_valhalla_trip(&alternate.trip, mode));
            }
        }

        PlanResponse {
            plan: Plan { itineraries },
            _otp: None,
            _valhalla: Some(valhalla),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Plan {
    itineraries: Vec<Itinerary>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Itinerary {
    mode: TravelMode,
    duration: f64,
    distance: f64,
    distance_units: DistanceUnit,
    legs: Vec<Leg>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Leg {
    /// encoded polyline. 1e-6 scale, (lat, lon)
    geometry: String,

    /// Some transit agencies have a color associated with their routes
    route_color: Option<String>,
    // mode: String,
    // from: Point,
    // to: Point,
    // distance: f64,
    // duration: f64,
}

impl Leg {
    fn from_otp(otp: &otp_api::Leg) -> Self {
        let line = polyline::decode_polyline(&otp.leg_geometry.points, 5).expect("TODO");
        let geometry = polyline::encode_coordinates(line, 6).expect("TODO");
        Self {
            geometry,
            route_color: otp.route_color.clone(),
        }
    }
    fn from_valhalla(valhalla: &valhalla_api::Leg) -> Self {
        Self {
            geometry: valhalla.shape.clone(),
            route_color: None,
        }
    }
}

impl Itinerary {
    fn from_valhalla_trip(valhalla: &valhalla_api::Trip, mode: TravelMode) -> Self {
        Self {
            mode,
            duration: valhalla.summary.time,
            distance: valhalla.summary.length,
            distance_units: valhalla.units,
            legs: valhalla.legs.iter().map(Leg::from_valhalla).collect(),
        }
    }
}

#[get("/v2/plan")]
pub async fn get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let Some(primary_mode) = query.mode.0.first() else {
        return Err(Error::user("mode is required"));
    };

    let distance_units = query
        .preferred_distance_units
        .unwrap_or(DistanceUnit::Kilometers);

    // FIXME: Handle bus+bike if bike is first
    match primary_mode {
        TravelMode::Transit => {
            let Some(mut router_url) = app_state
                .otp_cluster()
                .find_router_url(query.from_place, query.to_place)
            else {
                return Err(Error::user("no matching router found"));
            };

            // if we end up building this manually rather than passing it through, we'll need to be sure
            // to handle the bike+bus case
            router_url.set_query(Some(req.query_string()));
            log::debug!(
                "found matching router. Forwarding request to: {}",
                router_url
            );

            let otp_response: reqwest::Response = reqwest::get(router_url).await?;
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

            let otp_plan_response: otp_api::PlanResponse = otp_response.json().await?;
            let plan_response = PlanResponse::from_otp(*primary_mode, otp_plan_response);
            Ok(response.json(plan_response))
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
            let valhalla_response: reqwest::Response = reqwest::get(router_url).await?;
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

            let valhalla_route_response: valhalla_api::RouteResponse =
                valhalla_response.json().await?;
            let plan_response = PlanResponse::from_valhalla(*primary_mode, valhalla_route_response);
            Ok(response.json(plan_response))
        }
    }
}

// Comma separated list of travel modes
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
struct TravelModes(Vec<TravelMode>);

impl<'de> Deserialize<'de> for TravelModes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVecVisitor;

        impl<'de> Visitor<'de> for ColorVecVisitor {
            type Value = TravelModes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a comma-separated string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let modes = value
                    .split(',')
                    .map(|s| TravelMode::deserialize(s.into_deserializer()))
                    .collect::<Result<_, _>>()?;
                Ok(TravelModes(modes))
            }
        }

        deserializer.deserialize_str(ColorVecVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_route_walk.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response = PlanResponse::from_valhalla(TravelMode::Walk, valhalla);
        assert_eq!(plan_response.plan.itineraries.len(), 3);

        // itineraries
        let first_itinerary = &plan_response.plan.itineraries[0];
        assert_eq!(first_itinerary.mode, TravelMode::Walk);
        assert_relative_eq!(first_itinerary.distance, 9.148);
        assert_relative_eq!(first_itinerary.duration, 6488.443);

        // legs
        assert_eq!(first_itinerary.legs.len(), 1);
        let first_leg = &first_itinerary.legs[0];
        let geometry = polyline::decode_polyline(&first_leg.geometry, 6).unwrap();
        assert_relative_eq!(
            geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583),
            epsilon = 1e-4
        );
        assert_eq!(first_leg.route_color, None);
    }

    #[test]
    fn from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_plan_transit.json").unwrap();
        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();
        let plan_response = PlanResponse::from_otp(TravelMode::Transit, otp);

        assert_eq!(plan_response.plan.itineraries.len(), 5);

        // itineraries
        let first_itinerary = &plan_response.plan.itineraries[0];
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

        assert_eq!(first_leg.route_color, None);

        let fourth_leg = &first_itinerary.legs[3];
        assert_eq!(fourth_leg.route_color, Some("28813F".to_string()));
    }
}
