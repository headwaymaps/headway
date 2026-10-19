//! The generated style has to reproduce the checked-in Headway reference.

const REFERENCE: &str = include_str!("fixtures/headway.json");

#[test]
fn matches_the_checked_in_headway_style() {
    assert_eq!(maplibre_style::headway::style().to_json_string(), REFERENCE);
}
