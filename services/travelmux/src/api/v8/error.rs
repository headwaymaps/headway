use actix_web::body::BoxBody;
use actix_web::{HttpResponse, HttpResponseBuilder};
use serde::Serialize;
use std::fmt;

use super::Itinerary;
use crate::error::ErrorType;
use crate::otp::gtfs_graphql;
use crate::valhalla::valhalla_api;
use crate::{DistanceUnit, Error, TravelMode};

/// A successful plan.
///
/// Unlike v6, there's no `_otp`/`_valhalla` echo of the upstream response: everything a client
/// needs is in the itineraries themselves.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponseOk {
    pub(crate) itineraries: Vec<Itinerary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponseErr {
    pub error: PlanError,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanError {
    pub status_code: u16,
    pub error_code: u32,
    pub message: String,
}

impl fmt::Display for PlanResponseErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "status_code: {}, error_code: {}, message: {}",
            self.error.status_code, self.error.error_code, self.error.message
        )
    }
}

impl std::error::Error for PlanResponseErr {}

impl From<Error> for PlanResponseErr {
    fn from(value: Error) -> Self {
        Self {
            error: value.into(),
        }
    }
}

impl From<Error> for PlanError {
    fn from(value: Error) -> Self {
        let error_code = value.error_type as u32;
        let status_code = match value.error_type {
            ErrorType::NoCoverageForArea | ErrorType::User => 400,
            ErrorType::Server => 500,
        };
        Self {
            status_code,
            error_code,
            message: value.source.to_string(),
        }
    }
}

impl From<valhalla_api::RouteResponseError> for PlanResponseErr {
    fn from(value: valhalla_api::RouteResponseError) -> Self {
        Self {
            error: PlanError {
                status_code: value.status_code,
                // Errors originating in valhalla are offset by 2000
                error_code: value.error_code + 2000,
                message: value.error,
            },
        }
    }
}

impl From<&gtfs_graphql::RoutingError> for PlanResponseErr {
    /// OTP's routing errors are the user's problem, not the server's: they mean we understood the
    /// request but there's no trip to be had. `OUTSIDE_BOUNDS` and friends become
    /// [`ErrorType::NoCoverageForArea`], which is how callers know to try another router.
    fn from(value: &gtfs_graphql::RoutingError) -> Self {
        let error_type = if value.code.is_out_of_area() {
            ErrorType::NoCoverageForArea
        } else {
            ErrorType::User
        };
        Self {
            error: PlanError {
                status_code: 400,
                error_code: error_type as u32,
                message: format!("{}: {}", value.code.as_str(), value.description),
            },
        }
    }
}

impl actix_web::ResponseError for PlanResponseErr {
    fn status_code(&self) -> actix_web::http::StatusCode {
        self.error.status_code.try_into().unwrap_or_else(|e| {
            log::error!(
                "invalid status code: {}, err: {e:?}",
                self.error.status_code
            );
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
        })
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponseBuilder::new(self.status_code())
            .content_type("application/json")
            .json(self)
    }
}

impl PlanResponseOk {
    pub fn from_otp(
        mode: TravelMode,
        plan: gtfs_graphql::PlanResult,
        instruction_units: DistanceUnit,
    ) -> Result<PlanResponseOk, PlanResponseErr> {
        if let Some(routing_error) = plan.routing_error() {
            return Err(routing_error.into());
        }

        let mut itineraries = plan
            .itineraries
            .into_iter()
            .map(|itinerary| Itinerary::from_otp(itinerary, mode, instruction_units))
            .collect::<crate::Result<Vec<_>>>()?;
        itineraries.sort_by_key(|itinerary| itinerary.end_time);

        Ok(PlanResponseOk { itineraries })
    }

    pub fn from_valhalla(
        mode: TravelMode,
        valhalla: valhalla_api::ValhallaRouteResponseResult,
    ) -> Result<PlanResponseOk, PlanResponseErr> {
        let valhalla = match valhalla {
            valhalla_api::ValhallaRouteResponseResult::Ok(valhalla) => valhalla,
            valhalla_api::ValhallaRouteResponseResult::Err(err) => return Err(err.into()),
        };

        let mut itineraries = vec![Itinerary::from_valhalla(&valhalla.trip, mode)];
        if let Some(alternates) = &valhalla.alternates {
            for alternate in alternates {
                itineraries.push(Itinerary::from_valhalla(&alternate.trip, mode));
            }
        }

        Ok(PlanResponseOk { itineraries })
    }
}
