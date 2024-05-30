use super::error::{PlanResponseErr, PlanResponseOk};
use super::osrm_api;
use super::plan::{PlanQuery, _get_plan};
use crate::api::AppState;
use actix_web::{get, web, HttpRequest, HttpResponseBuilder};
use serde::Serialize;

/// Returns directions between locations in the format of the OSRM-ish API used by maplibre-directions.
#[get("/v6/directions")]
pub async fn get_directions(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<DirectionsResponseOk, PlanResponseErr> {
    let plan_response_ok = _get_plan(query, req, app_state).await?;
    Ok(plan_response_ok.into())
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

#[cfg(test)]
mod tests {
    use super::osrm_api;
    use super::PlanResponseOk;
    use super::*;
    use crate::otp::otp_api;
    use crate::valhalla::valhalla_api;
    use crate::{DistanceUnit, TravelMode};
    use approx::assert_relative_eq;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn directions_from_otp() {
        let stubbed_response =
            File::open("tests/fixtures/requests/opentripplanner_walk_plan.json").unwrap();

        let otp: otp_api::PlanResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let plan_response =
            PlanResponseOk::from_otp(TravelMode::Walk, otp, DistanceUnit::Miles).unwrap();

        let directions_response = DirectionsResponseOk::from(plan_response);
        assert_eq!(directions_response.routes.len(), 1);

        let first_route = &directions_response.routes[0];
        // distance is always in meters for OSRM responses
        assert_relative_eq!(first_route.distance, 9165.05);
        assert_relative_eq!(first_route.duration, 7505.0);
        assert_relative_eq!(
            first_route.geometry.0[0],
            geo::coord!(x: -122.33922, y: 47.57583)
        );
        assert_relative_eq!(
            first_route.geometry.0.last().unwrap(),
            &geo::coord!(x:-122.3472, y: 47.65104)
        );

        let legs = &first_route.legs;
        assert_eq!(legs.len(), 1);
        let first_leg = &legs[0];

        assert_eq!(first_leg.distance, 9165.05);
        assert_eq!(first_leg.duration, 7505.0);
        assert_eq!(
            first_leg.summary,
            "Aurora Avenue North, East Marginal Way South",
        );
        assert_eq!(first_leg.steps.len(), 23);

        let first_step = &first_leg.steps[0];
        assert_eq!(first_step.distance, 19.15);
        assert_eq!(first_step.duration, 15.681392900202399);
        assert_eq!(first_step.name, "East Marginal Way South");
        assert_eq!(first_step.mode, TravelMode::Walk);

        let banner_instructions = first_step.banner_instructions.as_ref().unwrap();
        assert_eq!(banner_instructions.len(), 1);
        let banner_instruction = &banner_instructions[0];
        assert_relative_eq!(banner_instruction.distance_along_geometry, 19.15);
        let primary = &banner_instruction.primary;
        assert_eq!(primary.text, "Turn right onto path.");

        let Some(osrm_api::BannerComponent::Text(first_component)) = &primary.components.first()
        else {
            panic!(
                "unexpected banner component: {:?}",
                primary.components.first()
            )
        };
        assert_eq!(
            first_component.text,
            Some("Turn right onto path.".to_string())
        );

        let step_maneuver = &first_step.maneuver;
        assert_eq!(
            step_maneuver.location,
            geo::point!(x: -122.3392181, y: 47.5758346)
        );
    }

    #[test]
    fn directions_from_valhalla() {
        let stubbed_response =
            File::open("tests/fixtures/requests/valhalla_pedestrian_route.json").unwrap();
        let valhalla: valhalla_api::RouteResponse =
            serde_json::from_reader(BufReader::new(stubbed_response)).unwrap();

        let valhalla_response_result = valhalla_api::ValhallaRouteResponseResult::Ok(valhalla);
        let plan_response =
            PlanResponseOk::from_valhalla(TravelMode::Walk, valhalla_response_result).unwrap();

        let directions_response = DirectionsResponseOk::from(plan_response);
        assert_eq!(directions_response.routes.len(), 3);

        let first_route = &directions_response.routes[0];
        // distance is always in meters for OSRM responses
        assert_relative_eq!(first_route.distance, 9147.48856);
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

        assert_eq!(first_leg.distance, 9147.48856);
        assert_eq!(first_leg.duration, 6488.443);
        assert_eq!(
            first_leg.summary,
            "Dexter Avenue, East Marginal Way South, Alaskan Way South"
        );
        assert_eq!(first_leg.steps.len(), 21);

        let first_step = &first_leg.steps[0];
        assert_eq!(first_step.distance, 17.70274);
        assert_eq!(first_step.duration, 13.567);
        assert_eq!(first_step.name, "East Marginal Way South");
        assert_eq!(first_step.mode, TravelMode::Walk);

        let banner_instructions = first_step.banner_instructions.as_ref().unwrap();
        assert_eq!(banner_instructions.len(), 1);
        let banner_instruction = &banner_instructions[0];
        assert_relative_eq!(banner_instruction.distance_along_geometry, 17.70274);
        let primary = &banner_instruction.primary;
        assert_eq!(primary.text, "Turn right onto the walkway.");

        let Some(osrm_api::BannerComponent::Text(first_component)) = &primary.components.first()
        else {
            panic!(
                "unexpected banner component: {:?}",
                primary.components.first()
            )
        };
        assert_eq!(
            first_component.text,
            Some("Turn right onto the walkway.".to_string())
        );

        let step_maneuver = &first_step.maneuver;
        assert_eq!(
            step_maneuver.location,
            geo::point!(x: -122.339216, y: 47.575836)
        );
    }
}
