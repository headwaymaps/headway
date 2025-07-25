use crate::api::v6::plan::Leg;
use crate::api::AppState;
use crate::util::serde_util::serialize_line_string_as_polyline6;
use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder, ResponseError};
use geo::geometry::LineString;
use polyline::decode_polyline;
use polyline::errors::PolylineError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ElevationResponseOk {
    #[serde(serialize_with = "serialize_line_string_as_polyline6")]
    sampled_geometry: LineString,
    elevation: Vec<i16>,
    total_climb_meters: i16,
    total_fall_meters: i16,
}

impl Responder for ElevationResponseOk {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> actix_web::HttpResponse {
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        response.content_type("application/json");
        response.json(self)
    }
}

#[derive(Debug, Error)]
enum ElevationResponseErr {
    #[error("Decoding polyline error: {0}")]
    Polyline(#[from] PolylineError),

    #[error("Elevation error: {0}")]
    Inner(#[from] Box<dyn std::error::Error>),
}

impl ResponseError for ElevationResponseErr {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ElevationResponseErr::Polyline(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ElevationResponseErr::Inner(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Deserialize)]
struct ElevationQuery {
    // #[serde(deserialize_with = "decode_polyline6"
    // TODO: many
    /// encoded polyline. 1e-6 scale, (lat, lon)
    path: String,
}

/// Calculate total climb and fall from elevation data
///
/// This sums up all the positive elevation changes (climb) and negative elevation changes (fall)
/// separately, which is useful for understanding the total effort required.
fn calculate_climb_and_fall(elevations: &[i16]) -> (i16, i16) {
    if elevations.len() < 2 {
        return (0, 0);
    }

    let mut total_climb = 0;
    let mut total_fall = 0;

    for window in elevations.windows(2) {
        let elevation_change = window[1] - window[0];
        if elevation_change > 0 {
            total_climb += elevation_change;
        } else {
            total_fall -= elevation_change;
        }
    }

    (total_climb, total_fall)
}

#[get("/v6/elevation")]
pub async fn get_elevation(
    query: web::Query<ElevationQuery>,
    _req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<ElevationResponseOk, ElevationResponseErr> {
    let geometry = decode_polyline(&query.path, Leg::GEOMETRY_PRECISION)?;
    let (sampled_geometry, elevation) =
        app_state.elevation().sample_elevations(&geometry, 100.0)?;

    // Calculate total climb and fall
    let (total_climb_meters, total_fall_meters) = calculate_climb_and_fall(&elevation);

    Ok(ElevationResponseOk {
        sampled_geometry,
        elevation,
        total_climb_meters,
        total_fall_meters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AppState;
    use actix_web::{test, web, App};
    use std::path::PathBuf;
    use url::Url;

    fn build_test_app_state() -> AppState {
        AppState::new(
            Url::parse("http://test:8002").unwrap(),
            PathBuf::from("tests/fixtures/low_res_elevation_tifs"),
        )
    }

    #[actix_web::test]
    async fn test_get_elevation_success() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_app_state()))
                .service(get_elevation),
        )
        .await;

        // Encoded polyline for a simple path within the test data coverage area
        // This represents a path in Seattle where we have elevation data
        let space_needle = geo::coord!(x: -122.3493, y: 47.6205);
        let queen_anne = geo::coord!(x:-122.35461, y: 47.63437);
        let encoded_path =
            polyline::encode_coordinates([space_needle, queen_anne], Leg::GEOMETRY_PRECISION)
                .unwrap();

        let req = test::TestRequest::get()
            .uri(&format!("/v6/elevation?path={encoded_path}"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let serde_json::Value::String(encoded_sampled_geometry) =
            json.get("sampledGeometry").expect("value missing")
        else {
            panic!("unexpected type for sampled geometry");
        };
        let sampled_geometry =
            decode_polyline(encoded_sampled_geometry, Leg::GEOMETRY_PRECISION).unwrap();
        assert_eq!(sampled_geometry.0.first().unwrap(), &space_needle);
        assert_eq!(sampled_geometry.0.last().unwrap(), &queen_anne);
        assert_eq!(sampled_geometry.0.len(), 17);

        let serde_json::Value::Array(elevation) = json.get("elevation").unwrap() else {
            panic!("unexpected type for elevation");
        };
        assert_eq!(elevation.len(), sampled_geometry.0.len());
        assert_eq!(elevation.first().unwrap().as_u64().unwrap(), 37);
        assert_eq!(elevation.last().unwrap().as_u64().unwrap(), 129);

        let total_climb = json.get("totalClimbMeters").unwrap().as_u64().unwrap();
        let total_fall = json.get("totalFallMeters").unwrap().as_u64().unwrap();
        assert_eq!(total_climb, 112);
        assert_eq!(total_fall, 20);
    }

    #[actix_web::test]
    async fn test_get_elevation_invalid_polyline() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_app_state()))
                .service(get_elevation),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/v6/elevation?path=invalid_polyline")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }
}
