// These tests assume the server is running (and an OTP server to back it)
// eventually it'd be nice to manage the setup as well...
// Startup OTP on (e.g.) port 9001
// Then startup travelmux `cargo run -- "https://valhalla:8002" "http://otp:9001/otp/routers"`
// then run these tests

#[cfg(feature = "integration-tests")]
mod integration_tests {
    static SERVER_ROOT: &str = "http://localhost:8000";

    #[test]
    fn basic_plan() {
        let url = format!("{SERVER_ROOT}/plan?fromPlace=47.575837%2C-122.339414&toPlace=47.651048%2C-122.347234&numItineraries=3&mode=TRANSIT");
        let response = reqwest::blocking::get(url).unwrap();
        assert!(response.status().is_success());
        let body = response.json::<serde_json::Value>().unwrap();
        // print!("{}", serde_json::to_string_pretty(&body).unwrap());
        // FRAGILE: the number of itineraries might change
        assert_eq!(body["plan"]["itineraries"].as_array().unwrap().len(), 3);
    }
}
