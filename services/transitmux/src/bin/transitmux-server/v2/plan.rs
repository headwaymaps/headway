use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use geo::geometry::Point;
use reqwest::header::{HeaderName, HeaderValue};
use serde::de::IntoDeserializer;
use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize};
use std::fmt;

use transitmux::valhalla::valhalla_api::ModeCosting;
use transitmux::{util::deserialize_point_from_lat_lon, Error, TravelMode};
use transitmux::otp::otp_api;

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

type ValhallaPlanResponse = serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
struct PlanResponse {
    // The raw response from the upstream OTP service
    _otp: Option<otp_api::PlanResponse>,

    // The raw response from the upstream Valhalla service
    _valhalla: Option<ValhallaPlanResponse>,

    plan: Plan
}

#[derive(Debug, Deserialize, Serialize)]
struct Plan {
    itineraries: Vec<Itinerary>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Itinerary {
    duration: f64,
    // legs: Vec<Leg>,
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
            Ok(response.json(PlanResponse {
                plan: Plan {
                    itineraries: otp_plan.plan.itineraries.iter().map( |itinerary| {
                        Itinerary {
                            duration: itinerary.duration,
                        }
                    }).collect()
                },
                _otp: Some(otp_plan),
                _valhalla: None,
            }))
        }
        other => {
            debug_assert!(query.mode.0.len() == 1, "valhalla only supports one mode");

            let mode = match other {
                TravelMode::Transit => unreachable!("handled above"),
                TravelMode::Bicycle => ModeCosting::Bicycle,
                TravelMode::Car => ModeCosting::Auto,
                TravelMode::Walk => ModeCosting::Pedestrian,
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

            Ok(response.json(PlanResponse {
                plan: Plan {
                    itineraries: vec![], // TODO: parse valhalla response
                },
                _otp: None,
                _valhalla: Some(valhalla_response.json().await?),
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
