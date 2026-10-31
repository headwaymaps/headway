use geo::Point;
use url::Url;

use super::valhalla_api::{
    Costing, CostingOptions, LocationType, RouteLocation, SnapToAnyRoad, ValhallaRouteQuery,
};
use crate::{DistanceUnit, Result};

#[derive(Debug, Clone)]
pub struct ValhallaRouter {
    endpoint: Url,
}

impl ValhallaRouter {
    pub fn new(endpoint: Url) -> Self {
        Self { endpoint }
    }

    pub fn plan_url(
        &self,
        source: Point,
        via: &[Point],
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
        let endpoint = |point: Point| RouteLocation {
            lon_lat: point.into(),
            r#type: LocationType::Break,
            snap: snap.clone(),
        };
        let through = |point: &Point| RouteLocation {
            lon_lat: (*point).into(),
            r#type: LocationType::Through,
            snap: None,
        };
        let mut locations = vec![endpoint(source)];
        locations.extend(via.iter().map(through));
        locations.push(endpoint(destination));

        let query = ValhallaRouteQuery {
            locations,
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
        let router = ValhallaRouter::new("http://valhalla:8002".parse().unwrap());
        let url = router
            .plan_url(
                Point::new(-122.3, 47.6),
                &[Point::new(-122.35, 47.65)],
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
                { "lon": -122.3, "lat": 47.6, "type": "break", "radius": 15, "rank_candidates": false },
                { "lon": -122.35, "lat": 47.65, "type": "through" },
                { "lon": -122.4, "lat": 47.7, "type": "break", "radius": 15, "rank_candidates": false },
            ])
        );
    }

    #[test]
    fn walking_routes_start_and_end_on_the_nearest_road() {
        assert_eq!(
            requested_locations(ModeCosting::Pedestrian),
            json!([
                { "lon": -122.3, "lat": 47.6, "type": "break" },
                { "lon": -122.35, "lat": 47.65, "type": "through" },
                { "lon": -122.4, "lat": 47.7, "type": "break" },
            ])
        );
    }
}
