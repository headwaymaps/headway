//! The generated styles have to reproduce the checked-in Headway references.

use maplibre_style::headway::{style, Version};

#[test]
fn v3_matches_the_checked_in_style() {
    assert_eq!(
        style(Version::V3).to_json_string(),
        include_str!("fixtures/headway.json")
    );
}

#[test]
fn v4_matches_the_checked_in_style() {
    assert_eq!(
        style(Version::V4).to_json_string(),
        include_str!("fixtures/headway-v4.json")
    );
}
