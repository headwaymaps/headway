use actix_web::{get, web, HttpRequest, HttpResponseBuilder, Responder};
use geo::geometry::Point;
use serde::{Deserialize, Deserializer, Serialize};

use transitmux::Error;

use crate::AppState;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanQuery {
    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    to_place: Point,

    #[serde(deserialize_with = "deserialize_point_from_lat_lon")]
    from_place: Point,
}

#[get("/plan")]
pub async fn get_plan(
    query: web::Query<PlanQuery>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let Some(mut router_url) = app_state.cluster().find_router_url(query.from_place, query.to_place) else {
        return Err(Error::user("no matching router found"));
    };

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
    if let Some(content_type) = otp_response.headers().get("content-type") {
        response.content_type(content_type);
    } else {
        log::warn!("upstream didn't specify content-type");
    }

    Ok(response.streaming(otp_response.bytes_stream()))
}

fn deserialize_point_from_lat_lon<'de, D>(deserializer: D) -> std::result::Result<Point, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = Deserialize::deserialize(deserializer)?;
    use std::str::FromStr;
    let mut iter = s.split(',').map(f64::from_str);

    let Some(lat_res) = iter.next() else {
        return Err(D::Error::custom("missing lat"));
    };
    let lat = lat_res.map_err(|e| D::Error::custom(format!("invalid lat: {e}")))?;

    let Some(lon_res) = iter.next() else {
        return Err(D::Error::custom("missing lon"));
    };
    let lon = lon_res.map_err(|e| D::Error::custom(format!("invalid lon: {e}")))?;

    if let Some(next) = iter.next() {
        return Err(D::Error::custom(format!(
            "found an extra param in lat,lon,???: {next:?}"
        )));
    }

    Ok(Point::new(lon, lat))
}
