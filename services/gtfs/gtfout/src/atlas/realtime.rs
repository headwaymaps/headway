//! Matching GTFS-RT feeds to the static feeds they update.

use super::dmfr::{Feed, Operator, RealtimeFeed};

use std::collections::{BTreeMap, BTreeSet};

/// Which streams a realtime feed publishes, labelled for people rather than for
/// OTP - the tags transit-zoner shows on a row.
pub fn stream_kinds(rt: &RealtimeFeed) -> Vec<&'static str> {
    [
        (&rt.trip_updates_url, "trip updates"),
        (&rt.vehicle_positions_url, "vehicle positions"),
        (&rt.alerts_url, "alerts"),
    ]
    .into_iter()
    .filter_map(|(url, label)| url.as_ref().map(|_| label))
    .collect()
}

/// Maps operator Onestop IDs to the feeds they associate.
pub(crate) struct Associations {
    operator_feeds: BTreeMap<String, BTreeSet<String>>,
}

impl Associations {
    pub(crate) fn build<'a>(feeds: impl Iterator<Item = &'a Feed>, operators: &[Operator]) -> Self {
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
pub(crate) fn realtime_by_static<'a>(
    static_ids: impl Iterator<Item = &'a str>,
    realtime: &[RealtimeFeed],
    associations: &Associations,
) -> BTreeMap<String, Vec<RealtimeFeed>> {
    let static_ids: BTreeSet<&str> = static_ids.collect();

    let mut by_static: BTreeMap<String, Vec<RealtimeFeed>> = BTreeMap::new();
    for rt in realtime {
        for sibling in associations.siblings(&rt.feed) {
            if static_ids.contains(sibling.as_str()) {
                by_static.entry(sibling).or_default().push(rt.clone());
            }
        }
    }
    by_static
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::dmfr::AssociatedFeed;

    fn feed(id: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            operators: vec![],

            authorization: None,
        }
    }

    fn rt_feed(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            feed: feed(id),
            alerts_url: None,
            trip_updates_url: None,
            vehicle_positions_url: None,
        }
    }

    fn publishing_trip_updates(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            trip_updates_url: Some("https://example.com/tu.pb".to_owned()),
            ..rt_feed(id)
        }
    }

    fn publishing_alerts(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            alerts_url: Some("https://example.com/a.pb".to_owned()),
            ..rt_feed(id)
        }
    }

    fn operator(onestop_id: &str, associated: &[&str]) -> Operator {
        Operator {
            onestop_id: onestop_id.to_owned(),
            name: None,
            short_name: None,

            associated_feeds: associated
                .iter()
                .map(|id| AssociatedFeed {
                    feed_onestop_id: Some((*id).to_owned()),
                })
                .collect(),
        }
    }

    fn join(statics: &[Feed], realtime: &[RealtimeFeed]) -> BTreeMap<String, Vec<RealtimeFeed>> {
        let associations = Associations::build(
            statics.iter().chain(realtime.iter().map(|rt| &rt.feed)),
            &[],
        );
        realtime_by_static(
            statics.iter().map(|f| f.id.as_str()),
            realtime,
            &associations,
        )
    }

    #[test]
    fn associates_through_a_shared_nested_operator() {
        let mut static_feed = feed("f-c23-kcm");
        static_feed.operators.push(operator("o-c23-kcm", &[]));

        let mut rt = publishing_trip_updates("f-c23-kcm~rt");
        rt.feed.operators.push(operator("o-c23-kcm", &[]));

        let by_static = join(&[static_feed], &[rt]);

        assert_eq!(by_static["f-c23-kcm"][0].feed.id, "f-c23-kcm~rt");
    }

    #[test]
    fn associates_through_explicit_associated_feeds() {
        let mut rt = publishing_alerts("f-c23-kcm~rt");
        rt.feed
            .operators
            .push(operator("o-c23-kcm", &["f-c23-kcm"]));

        let by_static = join(&[feed("f-c23-kcm")], &[rt]);

        assert_eq!(by_static["f-c23-kcm"][0].feed.id, "f-c23-kcm~rt");
    }

    #[test]
    fn an_rt_feed_covering_several_agencies_reaches_each_of_them() {
        let mut regional = publishing_trip_updates("f-c23-regional~rt");
        regional
            .feed
            .operators
            .push(operator("o-c23-regional", &["f-c23-kcm", "f-c23-st"]));

        let by_static = join(&[feed("f-c23-kcm"), feed("f-c23-st")], &[regional]);

        assert_eq!(
            by_static.keys().collect::<Vec<_>>(),
            ["f-c23-kcm", "f-c23-st"]
        );
        assert_eq!(by_static["f-c23-kcm"][0].feed.id, "f-c23-regional~rt");
        assert_eq!(by_static["f-c23-st"][0].feed.id, "f-c23-regional~rt");
    }

    #[test]
    fn an_rt_feed_with_no_operator_is_unreachable() {
        let by_static = join(&[feed("f-c23-kcm")], &[publishing_alerts("f-orphan~rt")]);

        assert!(by_static.is_empty());
    }
}
