use gtfout::feed_config::FeedConfig;
use gtfout::transit_zone::feed_id::feed_id_for;
use gtfout::transit_zone::router_config::{required_feeds, Scope};
use gtfout::transit_zone::zone::{Zone, VERSION};

use std::path::{Path, PathBuf};

fn committed_zones() -> Vec<PathBuf> {
    let builds = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../builds")
        .canonicalize()
        .expect("builds/ is where the config directories live");

    let mut found = Vec::new();
    for config in builds.read_dir().expect("reading builds/").flatten() {
        let Ok(entries) = config.path().join("transit").read_dir() else {
            continue; // a config with no transit
        };
        for zone in entries.flatten() {
            let zone_file = zone.path().join("zone.json");
            if zone_file.is_file() {
                found.push(zone_file);
            }
        }
    }
    assert!(
        !found.is_empty(),
        "no zone files under builds/*/transit/*/zone.json"
    );
    found.sort();
    found
}

#[test]
fn every_committed_zone_is_readable_by_the_build() {
    for path in committed_zones() {
        let zone = Zone::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(zone.version, VERSION);
        assert!(
            !zone.feeds.is_empty(),
            "{}: a zone with no feeds builds an empty graph",
            path.display()
        );
        assert!(
            zone.bounds.min_lon < zone.bounds.max_lon && zone.bounds.min_lat < zone.bounds.max_lat,
            "{}: bounds are inside out",
            path.display()
        );
        for feed in &zone.feeds {
            assert!(
                !feed.url.is_empty(),
                "{}: {} has no URL to download",
                path.display(),
                feed.feed_onestop_id
            );
        }
    }
}

#[test]
fn zone_directory_names_are_valid_k8s_object_names() {
    for path in committed_zones() {
        let name = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            name.len() <= 40
                && !name.starts_with('-')
                && !name.ends_with('-')
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{name}: not a valid k8s object name - lowercase letters, digits and dashes only"
        );
    }
}

#[test]
fn rendered_updaters_name_feeds_the_graph_will_have() {
    for path in committed_zones() {
        let zone = Zone::load(&path).unwrap();
        // Stand-in tokens, so the token-gated feeds render rather than skip.
        let credentials: FeedConfig = required_feeds(&zone, Scope::All)
            .into_iter()
            .map(|feed_id| (feed_id, "test-token".to_owned()))
            .collect();
        let (config, _skipped) = zone.router_config(&credentials);

        let built: Vec<String> = zone
            .feeds
            .iter()
            .map(|feed| feed_id_for(&feed.feed_onestop_id))
            .collect();

        for updater in &config.updaters {
            assert!(
                built.contains(&updater.feed_id),
                "{}: updater for {} matches no feed in the zone ({built:?})",
                path.display(),
                updater.feed_id
            );
            assert!(
                updater.url.starts_with("http"),
                "{}: {} has no URL to poll",
                path.display(),
                updater.feed_id
            );
        }
    }
}

#[test]
fn the_schema_refuses_a_credential() {
    let with_credential = r#"{
      "version": 1,
      "bounds": { "min_lon": -122.5, "min_lat": 47.3, "max_lon": -122.0, "max_lat": 47.8 },
      "feeds": [
        {
          "feed_onestop_id": "f-9q8y-sfmta",
          "provider": "SFMTA",
          "url": "https://example.com/sf.zip",
          "authorization": { "type": "query_param", "param_name": "api_key",
                             "credential": "a-token" }
        }
      ]
    }"#;

    let error = Zone::parse(with_credential)
        .expect_err("a zone carrying a credential must not parse")
        .to_string();
    assert!(error.contains("credential"), "{error}");
}
