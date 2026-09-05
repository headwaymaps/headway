//! The zone files in `builds/` are the picker's output and the build's input,
//! and nothing else in the test suite reads them: the schema tests use fixtures
//! they construct, and the build only finds out at deploy time.
//!
//! So this checks the real documents - that every one this repository ships
//! parses at the version the build understands, and renders the OTP config the
//! deployment pastes into a ConfigMap.

use transit_zone::feed_id::feed_id_for;
use transit_zone::zone::{Zone, VERSION};

use std::path::{Path, PathBuf};

/// Every `builds/<config>/transit/zones/<zone>.json` in the repository.
fn committed_zones() -> Vec<PathBuf> {
    let builds = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../builds")
        .canonicalize()
        .expect("builds/ is where the config directories live");

    let mut found = Vec::new();
    for config in builds.read_dir().expect("reading builds/").flatten() {
        let zones = config.path().join("transit/zones");
        let Ok(entries) = zones.read_dir() else {
            continue; // a config with no transit
        };
        for zone in entries.flatten() {
            if zone.path().extension().is_some_and(|ext| ext == "json") {
                found.push(zone.path());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn there_are_some_to_check() {
    // Otherwise a rename in builds/ would turn every test below into a no-op
    // that still passes.
    assert!(
        !committed_zones().is_empty(),
        "no zone files under builds/*/transit/zones - has the layout moved?"
    );
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

/// The zone name is the file name, and it reaches Kubernetes as
/// `opentripplanner-<zone>`. `bin/build-transit` lints this before a multi-hour
/// build; checking it here means a bad name never gets committed in the first
/// place.
#[test]
fn zone_file_names_are_valid_k8s_object_names() {
    for path in committed_zones() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
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

/// Every updater OTP polls has to name a feed the graph was built with, or it
/// updates nothing. Both sides come from this one document, so they agree by
/// construction - this is the test that says so.
#[test]
fn rendered_updaters_name_feeds_the_graph_will_have() {
    for path in committed_zones() {
        let zone = Zone::load(&path).unwrap();
        let (config, _skipped) = zone.router_config();

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

/// A committed zone shouldn't carry a credential: the point of the `${VAR}`
/// indirection is that the file is reviewable and the secret isn't in it.
#[test]
fn no_committed_zone_carries_a_credential() {
    for path in committed_zones() {
        let zone = Zone::load(&path).unwrap();
        for feed in &zone.feeds {
            let auths = feed.authorization.iter().chain(
                feed.realtime
                    .iter()
                    .filter_map(|rt| rt.authorization.as_ref()),
            );
            for auth in auths {
                assert!(
                    auth.credential.is_empty(),
                    "{}: {} has a credential written into it - keep that out of git",
                    path.display(),
                    feed.feed_onestop_id
                );
            }
        }
    }
}
