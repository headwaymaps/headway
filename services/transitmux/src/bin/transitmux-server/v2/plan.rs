use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use geo::geometry::Point;
use reqwest::header::{HeaderName, HeaderValue};
use serde::de::IntoDeserializer;
use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize};
use std::fmt;

use transitmux::otp::otp_api;
use transitmux::valhalla::valhalla_api;
use transitmux::{util::deserialize_point_from_lat_lon, Error, TravelMode};

use crate::AppState;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanQuery {
    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    to_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    from_place: Point,

    num_itineraries: u32,

    mode: TravelModes,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Plan {
    itineraries: Vec<Itinerary>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Itinerary {
    mode: TravelMode,
    duration: f64,
    distance_meters: f64,
    // legs: Vec<Leg>,
}

impl Itinerary {
    fn from_valhalla_trip(valhalla_trip: valhalla_api::Trip, mode: TravelMode) -> Self {
        debug_assert_eq!(valhalla_trip.units, valhalla_api::DistanceUnit::Kilometers);
        let distance_meters = valhalla_trip.summary.length * 1000.0;
        Self {
            mode,
            duration: valhalla_trip.summary.time,
            distance_meters
        }
    }
}

// #[derive(Debug, Deserialize, Serialize)]
// struct Leg {
//     mode: String,
//     from: Point,
//     to: Point,
//     distance: f64,
//     duration: f64,
// }

#[get("/v2/plan")]
pub async fn get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let Some(primary_mode) = query.mode.0.first() else {
        return Err(Error::user("mode is required"));
    };

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

            let otp_plan: otp_api::PlanResponse = otp_response.json().await?;
            let itineraries = otp_plan
                .plan
                .itineraries
                .iter()
                .map(|itinerary| Itinerary {
                    duration: itinerary.duration,
                    mode: *primary_mode,
                    distance_meters: itinerary.legs.iter().map(|l| l.distance).sum(),
                })
                .collect();

            Ok(response.json(PlanResponse {
                plan: Plan { itineraries },
                _otp: Some(otp_plan),
                _valhalla: None,
            }))
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

            let mut itineraries = vec![Itinerary::from_valhalla_trip(valhalla_route_response.trip.clone(), *primary_mode)];
            if let Some(alternates) = &valhalla_route_response.alternates {
                for alternate in alternates {
                    itineraries.push(
                        Itinerary::from_valhalla_trip(alternate.trip.clone(), *primary_mode)
                    );
                }
            }

            Ok(response.json(PlanResponse {
                plan: Plan { itineraries },
                _otp: None,
                _valhalla: Some(valhalla_route_response),
            }))
        }
    }
}

// Comma separated list of travel modes
#[derive(Debug, Serialize, PartialEq, Eq)]
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
