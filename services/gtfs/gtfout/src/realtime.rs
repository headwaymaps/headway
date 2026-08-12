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
//! several: typically a regional aggregator alongside a per-agency feed. A zone
//! nests such a feed under each of its in-scope static feeds rather than
//! guessing, since OTP keys by `feedId` and a stream covering several agencies
//! genuinely does update all of them.
//!
//! This is where the atlas stops. Once a zone file is written it carries the
//! endpoints itself, and `transit_zone::router_config` turns them into OTP's
//! updaters with no catalog in sight.

use crate::dmfr::{Feed, Operator};

use std::collections::{BTreeMap, BTreeSet};

use transit_zone::zone::RealtimeUrls;

/// The endpoints a realtime feed publishes, as a zone records them.
pub fn realtime_urls(feed: &Feed) -> RealtimeUrls {
    RealtimeUrls {
        trip_updates: feed.urls.realtime_trip_updates.clone(),
        vehicle_positions: feed.urls.realtime_vehicle_positions.clone(),
        alerts: feed.urls.realtime_alerts.clone(),
    }
}

/// Which streams a realtime feed publishes, labelled for people rather than for
/// OTP - the tags the picker shows on a row.
pub fn stream_kinds(feed: &Feed) -> Vec<&'static str> {
    let urls = realtime_urls(feed);

    [
        (urls.trip_updates, "trip updates"),
        (urls.vehicle_positions, "vehicle positions"),
        (urls.alerts, "alerts"),
    ]
    .into_iter()
    .filter_map(|(url, label)| url.map(|_| label))
    .collect()
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

/// The realtime feeds updating each static feed, keyed by static feed id.
///
/// RT feeds carry no stops, so they have no measured extent and can't be found
/// by a spatial query. They reach a zone only by hanging off a static feed that
/// can be, which is what this provides.
pub fn realtime_by_static<'a>(
    feeds: &'a [Feed],
    associations: &Associations,
) -> BTreeMap<String, Vec<&'a Feed>> {
    let static_ids: BTreeSet<&str> = feeds
        .iter()
        .filter(|f| f.is_gtfs())
        .map(|f| f.id.as_str())
        .collect();

    let mut by_static: BTreeMap<String, Vec<&Feed>> = BTreeMap::new();
    for rt in feeds.iter().filter(|f| f.is_gtfs_rt()) {
        // Siblings include other RT feeds of the same operator; only the static
        // ones are somewhere a zone can find them.
        for sibling in associations.siblings(rt) {
            if static_ids.contains(sibling.as_str()) {
                by_static.entry(sibling).or_default().push(rt);
            }
        }
    }
    by_static
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmfr::{AssociatedFeed, Urls};

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

    #[test]
    fn associates_through_a_shared_nested_operator() {
        let mut static_feed = feed("f-c23-kcm", "gtfs");
        static_feed.operators.push(operator("o-c23-kcm", &[]));

        let mut rt = feed("f-c23-kcm~rt", "gtfs-rt");
        rt.operators.push(operator("o-c23-kcm", &[]));
        rt.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let by_static = realtime_by_static(&feeds, &assoc);

        assert_eq!(by_static["f-c23-kcm"][0].id, "f-c23-kcm~rt");
    }

    #[test]
    fn associates_through_explicit_associated_feeds() {
        let static_feed = feed("f-c23-kcm", "gtfs");

        let mut rt = feed("f-c23-kcm~rt", "gtfs-rt");
        rt.operators.push(operator("o-c23-kcm", &["f-c23-kcm"]));
        rt.urls.realtime_alerts = Some("https://example.com/alerts.pb".to_owned());

        let feeds = vec![static_feed, rt];
        let assoc = Associations::build(&feeds, &[]);
        let by_static = realtime_by_static(&feeds, &assoc);

        assert_eq!(by_static["f-c23-kcm"][0].id, "f-c23-kcm~rt");
    }

    #[test]
    fn an_rt_feed_covering_several_agencies_reaches_each_of_them() {
        let mut regional = feed("f-c23-regional~rt", "gtfs-rt");
        regional
            .operators
            .push(operator("o-c23-regional", &["f-c23-kcm", "f-c23-st"]));
        regional.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());

        let feeds = vec![
            feed("f-c23-kcm", "gtfs"),
            feed("f-c23-st", "gtfs"),
            regional,
        ];
        let assoc = Associations::build(&feeds, &[]);
        let by_static = realtime_by_static(&feeds, &assoc);

        assert_eq!(
            by_static.keys().collect::<Vec<_>>(),
            ["f-c23-kcm", "f-c23-st"]
        );
    }

    #[test]
    fn an_rt_feed_with_no_operator_is_unreachable() {
        let mut rt = feed("f-orphan~rt", "gtfs-rt");
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let feeds = vec![feed("f-c23-kcm", "gtfs"), rt];
        let assoc = Associations::build(&feeds, &[]);

        assert!(realtime_by_static(&feeds, &assoc).is_empty());
    }

    #[test]
    fn static_feeds_are_never_treated_as_realtime() {
        let mut static_feed = feed("f-c23-kcm", "gtfs");
        static_feed.operators.push(operator("o-c23-kcm", &[]));
        // A static feed with an alerts URL is still not an RT feed.
        static_feed.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let feeds = vec![static_feed];
        let assoc = Associations::build(&feeds, &[]);

        assert!(realtime_by_static(&feeds, &assoc).is_empty());
    }

    #[test]
    fn stream_kinds_are_read_off_the_urls() {
        let mut rt = feed("f-x~rt", "gtfs-rt");
        rt.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        assert_eq!(stream_kinds(&rt), ["trip updates", "alerts"]);
    }
}
