//! Matching GTFS-RT feeds to the static feeds they update.
//!
//! OpenTripPlanner applies a realtime stream to a static feed by `feedId`, so
//! for every zone we need to know which RT endpoints belong to which of its
//! static feeds. DMFR records no direct link between the two - the association
//! runs through operators.
//!
//! An operator associates feeds two ways, and both count:
//!
//! - **explicitly**, via `associated_feeds[].feed_onestop_id`
//! - **implicitly**, by being nested inside a feed record, which associates
//!   that feed
//!
//! So an RT feed reaches its static counterpart by sharing an operator with it.
//! Across the catalog that resolves 644 of 666 RT feeds (97%); the remaining 22
//! have no operator at all and are unreachable by any means the atlas offers.
//!
//! Most RT feeds resolve to exactly one static feed, but 107 resolve to
//! several: typically a regional aggregator alongside a per-agency feed. We
//! emit an updater for each in-scope static feed rather than guessing, since
//! OTP keys by `feedId` and a stream covering several agencies genuinely does
//! update all of them.

use crate::dmfr::{Feed, Operator};
use crate::feed_id::feed_id_for;

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// The kinds of realtime stream OTP can consume, and how it names them.
///
/// Frequencies match what the old python emitted: alerts change slowly, trip
/// updates and vehicle positions don't.
const UPDATER_KINDS: &[(&str, &str, u32)] = &[
    ("realtime_alerts", "real-time-alerts", 300),
    ("realtime_trip_updates", "stop-time-updater", 60),
    ("realtime_vehicle_positions", "vehicle-positions", 60),
];

/// One entry in OTP's `router-config.json` `updaters` array.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Updater {
    /// Which static feed this stream updates, as stamped into its feed_info.txt.
    #[serde(rename = "feedId")]
    pub feed_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// OTP duration string, e.g. "60s".
    pub frequency: String,
    pub url: String,
}

/// An RT feed we can't emit an updater for, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRealtime {
    pub feed_id: String,
    pub reason: String,
}

/// Maps operator Onestop IDs to the feeds they associate.
///
/// Built once over the whole catalog, since an operator defined in one DMFR
/// file routinely associates feeds declared in another.
pub struct Associations {
    operator_feeds: BTreeMap<String, BTreeSet<String>>,
}

impl Associations {
    /// `operators` are the top-level ones; operators nested in a feed are read
    /// off the feeds themselves.
    pub fn build(feeds: &[Feed], operators: &[Operator]) -> Self {
        let mut operator_feeds: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        let mut record = |operator: &Operator, implicit: Option<&str>| {
            let entry = operator_feeds
                .entry(operator.onestop_id.clone())
                .or_default();
            if let Some(feed_id) = implicit {
                entry.insert(feed_id.to_owned());
            }
            for associated in &operator.associated_feeds {
                if let Some(id) = &associated.feed_onestop_id {
                    entry.insert(id.clone());
                }
            }
        };

        for feed in feeds {
            for operator in &feed.operators {
                // Nesting an operator in a feed implicitly associates it.
                record(operator, Some(&feed.id));
            }
        }
        for operator in operators {
            record(operator, None);
        }

        Self { operator_feeds }
    }

    /// The feeds sharing an operator with `feed`, excluding itself.
    fn siblings(&self, feed: &Feed) -> BTreeSet<String> {
        let mut siblings = BTreeSet::new();

        for operator in &feed.operators {
            if let Some(feeds) = self.operator_feeds.get(&operator.onestop_id) {
                siblings.extend(feeds.iter().cloned());
            }
        }
        // Also catch operators declared elsewhere that point back at this feed.
        for feeds in self.operator_feeds.values() {
            if feeds.contains(&feed.id) {
                siblings.extend(feeds.iter().cloned());
            }
        }

        siblings.remove(&feed.id);
        siblings
    }
}

/// Builds the OTP updaters for a zone.
///
/// `static_feed_ids` are the curated feeds in the zone - only RT streams
/// belonging to one of those are emitted, so deleting a row from the curated
/// CSV drops its realtime config too.
///
/// Authenticated RT feeds are reported rather than emitted. We don't inline
/// credentials into router-config.json: it's a committed artifact that ends up
/// in a k8s ConfigMap, so a key in it would be a key in git.
pub fn updaters_for(
    feeds: &[Feed],
    static_feed_ids: &BTreeSet<String>,
    associations: &Associations,
) -> (Vec<Updater>, Vec<SkippedRealtime>) {
    let mut updaters = Vec::new();
    let mut skipped = Vec::new();

    for feed in feeds.iter().filter(|f| f.is_gtfs_rt()) {
        let targets: Vec<String> = associations
            .siblings(feed)
            .into_iter()
            .filter(|id| static_feed_ids.contains(id))
            .collect();

        if targets.is_empty() {
            continue;
        }

        if let Some(auth) = &feed.authorization {
            skipped.push(SkippedRealtime {
                feed_id: feed.id.clone(),
                reason: format!(
                    "needs {} authentication; router-config.json is committed, so the key can't be baked in",
                    auth.kind
                ),
            });
            continue;
        }

        for (url_field, kind, frequency_sec) in UPDATER_KINDS {
            let Some(url) = feed.urls.realtime_url(url_field) else {
                continue;
            };
            for target in &targets {
                updaters.push(Updater {
                    feed_id: feed_id_for(target),
                    kind: (*kind).to_owned(),
                    frequency: format!("{frequency_sec}s"),
                    url: url.to_owned(),
                });
            }
        }
    }

    updaters.sort();
    updaters.dedup();
    skipped.sort_by(|a, b| a.feed_id.cmp(&b.feed_id));
    skipped.dedup();
    (updaters, skipped)
}

/// The `router-config.json` document OTP reads.
#[derive(Debug, Serialize)]
pub struct RouterConfig {
    pub updaters: Vec<Updater>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmfr::{AssociatedFeed, Authorization, Operator, Urls};

    use std::collections::BTreeMap as Map;

    fn feed(id: &str, spec: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            spec: spec.to_owned(),
            urls: Urls::default(),
            operators: vec![],
            tags: Map::new(),
            authorization: None,
            source_domain: "example.com".to_owned(),
        }
    }

    fn operator(onestop_id: &str, associated: &[&str]) -> Operator {
        Operator {
            onestop_id: onestop_id.to_owned(),
            name: None,
            short_name: None,
            website: None,
            associated_feeds: associated
                .iter()
                .map(|id| AssociatedFeed {
                    feed_onestop_id: Some((*id).to_owned()),
                })
                .collect(),
            tags: Map::new(),
        }
    }

    fn in_scope(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn associates_through_a_shared_nested_operator() {
        let mut static_feed = feed("f-c23-kcm", "gtfs");
        static_feed.operators.push(operator("o-c23-kcm", &[]));

        let mut rt = feed("f-c23-kcm~rt", "gtfs-rt");
        rt.operators.push(operator("o-c23-kcm", &[]));
        rt.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, skipped) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        assert!(skipped.is_empty());
        assert_eq!(
            updaters,
            [Updater {
                feed_id: "headway-f-c23-kcm".to_owned(),
                kind: "stop-time-updater".to_owned(),
                frequency: "60s".to_owned(),
                url: "https://example.com/tu.pb".to_owned(),
            }]
        );
    }

    #[test]
    fn associates_through_explicit_associated_feeds() {
        let static_feed = feed("f-c23-kcm", "gtfs");

        let mut rt = feed("f-c23-kcm~rt", "gtfs-rt");
        rt.operators.push(operator("o-c23-kcm", &["f-c23-kcm"]));
        rt.urls.realtime_alerts = Some("https://example.com/alerts.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        assert_eq!(updaters.len(), 1);
        assert_eq!(updaters[0].kind, "real-time-alerts");
        assert_eq!(updaters[0].frequency, "300s", "alerts poll slowly");
    }

    #[test]
    fn emits_one_updater_per_stream_kind() {
        let mut static_feed = feed("f-c23-kcm", "gtfs");
        static_feed.operators.push(operator("o-c23-kcm", &[]));

        let mut rt = feed("f-c23-kcm~rt", "gtfs-rt");
        rt.operators.push(operator("o-c23-kcm", &[]));
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());
        rt.urls.realtime_trip_updates = Some("https://example.com/t.pb".to_owned());
        rt.urls.realtime_vehicle_positions = Some("https://example.com/v.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        let kinds: BTreeSet<&str> = updaters.iter().map(|u| u.kind.as_str()).collect();
        assert_eq!(
            kinds,
            BTreeSet::from(["real-time-alerts", "stop-time-updater", "vehicle-positions"])
        );
    }

    #[test]
    fn ignores_rt_feeds_for_static_feeds_outside_the_zone() {
        // The curated CSV is the authority: drop a feed from it and its
        // realtime config should disappear too.
        let mut static_feed = feed("f-9q8y-sfmta", "gtfs");
        static_feed.operators.push(operator("o-9q8y-sfmta", &[]));

        let mut rt = feed("f-9q8y-sfmta~rt", "gtfs-rt");
        rt.operators.push(operator("o-9q8y-sfmta", &[]));
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        assert!(updaters.is_empty());
    }

    #[test]
    fn an_rt_feed_covering_several_agencies_updates_each_in_scope_one() {
        let mut regional = feed("f-c23-regional~rt", "gtfs-rt");
        regional.operators.push(operator(
            "o-c23-regional",
            &["f-c23-kcm", "f-c23-st", "f-9q8y-sfmta"],
        ));
        regional.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());

        let feeds = vec![
            feed("f-c23-kcm", "gtfs"),
            feed("f-c23-st", "gtfs"),
            feed("f-9q8y-sfmta", "gtfs"),
            regional,
        ];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm", "f-c23-st"]), &assoc);

        let feed_ids: Vec<&str> = updaters.iter().map(|u| u.feed_id.as_str()).collect();
        assert_eq!(feed_ids, ["headway-f-c23-kcm", "headway-f-c23-st"]);
    }

    #[test]
    fn skips_authenticated_rt_feeds_rather_than_baking_in_a_key() {
        let mut static_feed = feed("f-9q9-actransit", "gtfs");
        static_feed.operators.push(operator("o-9q9-actransit", &[]));

        let mut rt = feed("f-9q9-actransit~rt", "gtfs-rt");
        rt.operators.push(operator("o-9q9-actransit", &[]));
        rt.urls.realtime_alerts = Some("https://api.511.org/alerts".to_owned());
        rt.authorization = Some(Authorization {
            kind: "query_param".to_owned(),
            param_name: Some("api_key".to_owned()),
            info_url: None,
        });

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, skipped) = updaters_for(&feeds, &in_scope(&["f-9q9-actransit"]), &assoc);

        assert!(updaters.is_empty());
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].feed_id, "f-9q9-actransit~rt");
        assert!(skipped[0].reason.contains("query_param"), "{skipped:?}");
    }

    #[test]
    fn an_rt_feed_with_no_operator_yields_nothing() {
        let mut rt = feed("f-orphan~rt", "gtfs-rt");
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let feeds = vec![feed("f-c23-kcm", "gtfs"), rt];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        assert!(updaters.is_empty());
    }

    #[test]
    fn static_feeds_never_become_updaters() {
        let mut static_feed = feed("f-c23-kcm", "gtfs");
        static_feed.operators.push(operator("o-c23-kcm", &[]));
        // A static feed with an alerts URL is still not an RT feed.
        static_feed.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let feeds = vec![static_feed];
        let assoc = Associations::build(&feeds, &[]);
        let (updaters, _) = updaters_for(&feeds, &in_scope(&["f-c23-kcm"]), &assoc);

        assert!(updaters.is_empty());
    }
}
