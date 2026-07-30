// These tests assume the server is running (and an OTP server to back it)
// eventually it'd be nice to manage the setup as well...
// Startup OTP on (e.g.) port 9001
// Then startup travelmux `cargo run -- "https://valhalla:8002" "http://otp:9001"`
// then run these tests

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use serde_json::Value;

    static SERVER_ROOT: &str = "http://localhost:8000/v7";

    fn get_plan(query: &str) -> Value {
        let url = format!("{SERVER_ROOT}/plan?{query}");
        let response = reqwest::blocking::get(url).unwrap();
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap();
            panic!("status was: {status}, body: {body}");
        }
        response.json().unwrap()
    }

    fn itineraries(body: &Value) -> &Vec<Value> {
        body["itineraries"].as_array().unwrap()
    }

    /// Every leg is either a transit leg or a non-transit leg, never both.
    fn assert_legs_well_formed(itinerary: &Value) {
        for leg in itinerary["legs"].as_array().unwrap() {
            assert!(leg["startTime"].as_str().unwrap().contains('T'));
            assert!(leg["distanceMeters"].is_number());
            assert!(leg["durationSeconds"].is_number());
            assert_ne!(
                leg["transitLeg"].is_null(),
                leg["nonTransitLeg"].is_null(),
                "expected exactly one of transitLeg/nonTransitLeg: {leg}"
            );
        }
    }

    #[test]
    fn get_transit_plan() {
        let body = get_plan("fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=TRANSIT");
        let itineraries = itineraries(&body);
        assert!(1 < itineraries.len());

        let first = &itineraries[0];
        assert_eq!(first["mode"].as_str().unwrap(), "TRANSIT");
        // RFC 3339, in the graph's timezone
        assert!(first["startTime"].as_str().unwrap().contains('T'));
        assert_legs_well_formed(first);

        // A transit trip has at least one ride on a vehicle
        let transit_legs: Vec<_> = itineraries
            .iter()
            .flat_map(|itinerary| itinerary["legs"].as_array().unwrap())
            .filter(|leg| !leg["transitLeg"].is_null())
            .collect();
        assert!(!transit_legs.is_empty());
        assert!(transit_legs[0]["transitLeg"]["vehicleMode"].is_string());
    }

    #[test]
    fn bad_mode() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=FAKE_MODE");
        let response = reqwest::blocking::get(url).unwrap();
        assert!(!response.status().is_success());

        let body = response.text().unwrap();
        assert!(body.contains("unknown variant `FAKE_MODE`"));
    }

    #[test]
    fn bad_date_time() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=TRANSIT&dateTime=noon");
        let response = reqwest::blocking::get(url).unwrap();
        assert!(!response.status().is_success());

        let body = response.text().unwrap();
        assert!(body.contains("RFC 3339"));
    }

    #[test]
    fn get_local_walk_plan() {
        let body = get_plan("fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=2&mode=WALK");

        // Walking uses OTP where available, which returns a single itinerary
        let itineraries = itineraries(&body);
        assert_eq!(1, itineraries.len());
        assert_eq!(itineraries[0]["mode"].as_str().unwrap(), "WALK");
        assert_legs_well_formed(&itineraries[0]);
    }

    #[test]
    fn get_distant_walk_plan() {
        // Null island is outside the coverage of our OTP instance, so this is handled by valhalla
        // - which has no route there either, so all we can check is that we get an error rather
        // than a panic.
        let url = format!("{SERVER_ROOT}/plan?fromPlace=0.1%2C0.1&toPlace=0.101%2C0.101&numItineraries=2&mode=WALK");
        let response = reqwest::blocking::get(url).unwrap();
        let status = response.status();
        let body: Value = response.json().unwrap();
        if status.is_success() {
            assert!(!itineraries(&body).is_empty());
        } else {
            // Errors originating in valhalla are offset by 2000
            assert!(body["error"]["errorCode"].as_u64().unwrap() > 2000);
        }
    }
}
