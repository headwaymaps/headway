use crate::otp::otp_api;
use crate::valhalla::valhalla_api;
use crate::{DistanceUnit, Error, TravelMode};
use actix_web::HttpResponseBuilder;
use serde::{Deserialize, Serialize};

use super::{Itinerary, Plan};
use crate::error::ErrorType;
use actix_web::body::BoxBody;
use actix_web::HttpResponse;
use std::fmt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnboxedPlanResponseErr {
    pub error: PlanError,
    // The raw response from the upstream OTP /plan service
    #[serde(rename = "_otp")]
    _otp: Option<otp_api::PlanError>,

    // The raw response from the upstream Valhalla /route service
    #[serde(rename = "_valhalla")]
    _valhalla: Option<valhalla_api::RouteResponseError>,
}
pub type PlanResponseErr = Box<UnboxedPlanResponseErr>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanError {
    pub status_code: u16,
    pub error_code: u32,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponseOk {
    pub(crate) plan: Plan,

    // The raw response from the upstream OTP /plan service
    #[serde(rename = "_otp")]
    _otp: Option<otp_api::PlanResponse>,

    // The raw response from the upstream Valhalla /route service
    #[serde(rename = "_valhalla")]
    _valhalla: Option<valhalla_api::RouteResponse>,
}

impl From<valhalla_api::RouteResponseError> for PlanResponseErr {
    fn from(value: valhalla_api::RouteResponseError) -> Self {
        Self::new(UnboxedPlanResponseErr {
            error: (&value).into(),
            _valhalla: Some(value),
            _otp: None,
        })
    }
}

impl From<otp_api::PlanError> for PlanResponseErr {
    fn from(value: otp_api::PlanError) -> Self {
        Self::new(UnboxedPlanResponseErr {
            error: (&value).into(),
            _valhalla: None,
            _otp: Some(value),
        })
    }
}

impl From<&valhalla_api::RouteResponseError> for PlanError {
    fn from(value: &valhalla_api::RouteResponseError) -> Self {
        PlanError {
            status_code: value.status_code,
            error_code: value.error_code + 2000,
            message: value.error.clone(),
        }
    }
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
        Self::new(UnboxedPlanResponseErr {
            error: value.into(),
            _valhalla: None,
            _otp: None,
        })
    }
}

impl From<Error> for PlanError {
    fn from(value: Error) -> Self {
        let error_code = value.error_type as u32;
        match value.error_type {
            ErrorType::NoCoverageForArea => Self {
                status_code: 400,
                error_code,
                message: value.source.to_string(),
            },
            ErrorType::User => Self {
                status_code: 400,
                error_code,
                message: value.source.to_string(),
            },
            ErrorType::Server => Self {
                status_code: 500,
                error_code,
                message: value.source.to_string(),
            },
        }
    }
}

impl From<&otp_api::PlanError> for PlanError {
    fn from(value: &otp_api::PlanError) -> Self {
        Self {
            // This might be overzealous, but anecdotally, I haven't encountered any 500ish
            // errors with OTP surfaced in this way yet
            status_code: 400,
            error_code: value.id,
            message: value.msg.clone(),
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
        mut otp: otp_api::PlanResponse,
        distance_unit: DistanceUnit,
    ) -> Result<PlanResponseOk, PlanResponseErr> {
        if let Some(otp_error) = otp.error {
            return Err(otp_error.into());
        }

        otp.plan
            .itineraries
            .sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let itineraries_result: crate::Result<Vec<_>> = otp
            .plan
            .itineraries
            .iter()
            .map(|itinerary: &otp_api::Itinerary| {
                Itinerary::from_otp(itinerary, mode, distance_unit)
            })
            .collect();

        let itineraries = itineraries_result?;

        Ok(PlanResponseOk {
            plan: Plan { itineraries },
            _otp: Some(otp),
            _valhalla: None,
        })
    }

    pub fn from_valhalla(
        mode: TravelMode,
        valhalla: valhalla_api::ValhallaRouteResponseResult,
    ) -> Result<PlanResponseOk, valhalla_api::RouteResponseError> {
        let valhalla = match valhalla {
            valhalla_api::ValhallaRouteResponseResult::Ok(valhalla) => valhalla,
            valhalla_api::ValhallaRouteResponseResult::Err(err) => return Err(err),
        };

        let mut itineraries = vec![Itinerary::from_valhalla(&valhalla.trip, mode)];
        if let Some(alternates) = &valhalla.alternates {
            for alternate in alternates {
                itineraries.push(Itinerary::from_valhalla(&alternate.trip, mode));
            }
        }

        Ok(PlanResponseOk {
            plan: Plan { itineraries },
            _otp: None,
            _valhalla: Some(valhalla),
        })
    }
}
