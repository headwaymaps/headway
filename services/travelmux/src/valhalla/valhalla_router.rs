use geo::algorithm::Contains;
use geo::geometry::{MultiPolygon, Point, Polygon};
use serde::Deserialize;
use url::Url;

use super::valhalla_api::{
    Costing, CostingOptions, RouteLocation, SnapToAnyRoad, ValhallaRouteQuery,
};
use crate::{DistanceUnit, Error, Result};

/// A Valhalla instance, and the area its tiles cover.
#[derive(Debug, Clone)]
pub struct ValhallaRouter {
    id: String,
    endpoint: Url,
    coverage: MultiPolygon,
}

/// The part of Valhalla's verbose `/status` we read.
#[derive(Deserialize)]
struct Status {
    /// The outline of the tiles loaded, absent unless the service allows verbose status.
    bbox: Option<geojson::FeatureCollection>,
}

impl ValhallaRouter {
    pub fn new(id: String, endpoint: Url, coverage: MultiPolygon) -> Self {
        Self {
            id,
            endpoint,
            coverage,
        }
    }

    /// Learns the area `endpoint`'s tiles cover from its status.
    pub async fn fetch(id: String, endpoint: Url) -> Result<Self> {
        let mut status_url = endpoint.clone();
        status_url.set_path("/status");
        status_url
            .query_pairs_mut()
            .append_pair("json", r#"{"verbose":true}"#);
        let status: Status = reqwest::get(status_url)
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| Error::server(format!("fetching valhalla {endpoint} status: {e}")))?
            .json()
            .await
            .map_err(|e| Error::server(format!("reading valhalla {endpoint} status: {e}")))?;
        let Some(bbox) = status.bbox else {
            return Err(Error::server(format!(
                "valhalla {endpoint} doesn't report its coverage; set service_limits.status.allow_verbose"
            )));
        };
        let polygons = bbox
            .features
            .into_iter()
            .filter_map(|feature| feature.geometry)
            .map(|geometry| {
                Polygon::try_from(geometry).map_err(|e| {
                    Error::server(format!("valhalla {endpoint} coverage isn't polygons: {e}"))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::new(id, endpoint, MultiPolygon::new(polygons)))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    #[cfg(test)]
    pub(crate) fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn contains(&self, point: &Point) -> bool {
        self.coverage.contains(point)
    }

    pub fn plan_url(
        &self,
        source: Point,
        destination: Point,
        costing: Costing,
        num_itineraries: u32,
        distance_units: DistanceUnit,
    ) -> Result<Url> {
        let mut url = self.endpoint.clone();

        // A rider's start or end is often closest to a sidewalk, which is mapped as its own way
        // beside the street. Snapping to it routes them along the sidewalk to the nearest
        // crossing and back, where a real rider would just set off from (or stop at) the curb.
        // Letting Valhalla pick any road nearby by cost picks the street instead.
        let mode = costing.mode();
        let (snap, costing_options) = match costing {
            Costing::Bicycle(bicycle) => (
                Some(SnapToAnyRoad {
                    radius: 15,
                    rank_candidates: false,
                }),
                Some(CostingOptions { bicycle }),
            ),
            Costing::Auto | Costing::Pedestrian => (None, None),
        };
        let location = |point: Point| RouteLocation {
            lon_lat: point.into(),
            snap: snap.clone(),
        };

        let query = ValhallaRouteQuery {
            locations: vec![location(source), location(destination)],
            elevation_interval: mode.elevation_interval(),
            costing: mode,
            alternates: num_itineraries,
            // NOTE: these units get embedded in the localized turn-by-turn direction strings
            units: distance_units,
            costing_options,
        };

        url.set_path("/route");

        let json_string = serde_json::to_string(&query).expect("valid json representation");
        url.query_pairs_mut().append_pair("json", &json_string);

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::valhalla::valhalla_api::{BicycleCostingOptions, ModeCosting};
    use serde_json::{json, Value};

    fn requested(mode: ModeCosting) -> Value {
        requested_costing(mode.into())
    }

    fn requested_costing(costing: Costing) -> Value {
        let router = ValhallaRouter::new(
            "valhalla".to_string(),
            "http://valhalla:8002".parse().unwrap(),
            MultiPolygon::empty(),
        );
        let url = router
            .plan_url(
                Point::new(-122.3, 47.6),
                Point::new(-122.4, 47.7),
                costing,
                1,
                DistanceUnit::Kilometers,
            )
            .unwrap();
        let (_, query) = url.query_pairs().find(|(key, _)| key == "json").unwrap();
        serde_json::from_str(&query).unwrap()
    }

    fn requested_locations(mode: ModeCosting) -> Value {
        requested(mode)["locations"].clone()
    }

    #[test]
    fn bike_routes_send_their_costing() {
        let faster = requested_costing(Costing::Bicycle(BicycleCostingOptions::FASTER));
        assert_eq!(faster["costing"], json!("bicycle"));
        assert_eq!(
            faster["costing_options"]["bicycle"]["use_hills"],
            json!(1.0)
        );
        assert_eq!(
            requested(ModeCosting::Pedestrian)["costing_options"],
            Value::Null
        );
    }

    #[test]
    fn bike_routes_start_and_end_on_any_nearby_road() {
        assert_eq!(
            requested_locations(ModeCosting::Bicycle),
            json!([
                { "lon": -122.3, "lat": 47.6, "radius": 15, "rank_candidates": false },
                { "lon": -122.4, "lat": 47.7, "radius": 15, "rank_candidates": false },
            ])
        );
    }

    #[test]
    fn walking_routes_start_and_end_on_the_nearest_road() {
        assert_eq!(
            requested_locations(ModeCosting::Pedestrian),
            json!([{ "lon": -122.3, "lat": 47.6 }, { "lon": -122.4, "lat": 47.7 }])
        );
    }
}
