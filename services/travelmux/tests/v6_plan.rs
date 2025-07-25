// These tests assume the server is running (and an OTP server to back it)
// eventually it'd be nice to manage the setup as well...
// Startup OTP on (e.g.) port 9001
// Then startup travelmux `cargo run -- "https://valhalla:8002" "http://otp:9001/otp/routers"`
// then run these tests

#[cfg(feature = "integration-tests")]
mod integration_tests {
    static SERVER_ROOT: &str = "http://localhost:8000/v6";

    #[test]
    fn get_transit_plan() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=TRANSIT");
        let response = reqwest::blocking::get(url).unwrap();
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap();
            panic!("status was: {status}, body: {body}");
        }
        let body = response.json::<serde_json::Value>().unwrap();
        assert!(
            1 < body["_otp"]["plan"]["itineraries"]
                .as_array()
                .unwrap()
                .len()
        );
        assert!(body["_valhalla"].is_null());
    }

    #[test]
    fn bad_mode() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=FAKE_MODE");
        let response = reqwest::blocking::get(url).unwrap();
        let status = response.status();
        assert!(!status.is_success());

        let body = response.text().unwrap();
        assert!(body.contains("unknown variant `FAKE_MODE`"));
    }

    #[test]
    fn get_local_walk_plan() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=2&mode=WALK");
        let response = reqwest::blocking::get(url).unwrap();
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap();
            panic!("status was: {status}, body: {body}");
        }
        let body = response.json::<serde_json::Value>().unwrap();

        // Walking uses OTP where available
        assert_eq!(
            1,
            body["_otp"]["plan"]["itineraries"]
                .as_array()
                .unwrap()
                .len()
        );
        assert!(body["_valhalla"].is_null());
    }

    #[test]
    fn get_distant_walk_plan() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=0.1%2C0.1&toPlace=0.101%2C0.101&numItineraries=2&mode=WALK");
        let response = reqwest::blocking::get(url).unwrap();

        // Request will fail as there's no route on null island
        let body = response.json::<serde_json::Value>().unwrap();

        // But in any case, it should be handled by valhalla, not OTP since it's out of bounds of our OTP node
        assert!(body["_otp"].is_null());
        assert!(!body["_valhalla"].is_null());
    }

    #[test]
    fn elevation_test() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=0.1%2C0.1&toPlace=0.101%2C0.101&numItineraries=2&mode=WALK");
        let response = reqwest::blocking::get(url).unwrap();

        // Request will fail as there's no route on null island
        let body = response.json::<serde_json::Value>().unwrap();

        // But in any case, it should be handled by valhalla, not OTP since it's out of bounds of our OTP node
        assert!(body["_otp"].is_null());
        assert!(!body["_valhalla"].is_null());
    }
}
