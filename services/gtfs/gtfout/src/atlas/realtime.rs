//! Matching GTFS-RT feeds to the static feeds they update.

use super::dmfr::{FeedCore, FeedId, Operator, RealtimeFeed};

use std::collections::{BTreeMap, BTreeSet};

/// Which feeds each operator draws data from, keyed by operator Onestop ID.
struct OperatorFeeds {
    by_operator: BTreeMap<String, BTreeSet<FeedId>>,
}

impl OperatorFeeds {
    fn new<'a>(feeds: impl Iterator<Item = &'a FeedCore>, operators: &[Operator]) -> Self {
        let mut by_operator: BTreeMap<String, BTreeSet<FeedId>> = BTreeMap::new();

        let mut record = |operator: &Operator, implicit: Option<&FeedId>| {
            let entry = by_operator.entry(operator.onestop_id.clone()).or_default();
            // A feed listing an operator inline is itself one of that
            // operator's feeds, whether or not it says so again.
            if let Some(feed_id) = implicit {
                entry.insert(feed_id.clone());
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

        Self { by_operator }
    }

    /// The feeds sharing an operator with `feed`, excluding itself.
    fn siblings(&self, feed: &FeedCore) -> BTreeSet<FeedId> {
        let mut siblings = BTreeSet::new();

        // Operators the feed names itself...
        for operator in &feed.operators {
            if let Some(feeds) = self.by_operator.get(&operator.onestop_id) {
                siblings.extend(feeds.iter().cloned());
            }
        }
        // ...and operators that name the feed, which is how a top-level
        // operator record reaches a feed that never mentions it.
        for feeds in self.by_operator.values() {
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
/// The only link we have is via operator
pub(crate) fn realtime_by_static<'a>(
    statics: impl Iterator<Item = &'a FeedCore>,
    realtime: &[RealtimeFeed],
    operators: &[Operator],
) -> BTreeMap<FeedId, Vec<RealtimeFeed>> {
    let statics: Vec<&FeedCore> = statics.collect();
    let operator_feeds = OperatorFeeds::new(
        statics
            .iter()
            .copied()
            .chain(realtime.iter().map(|rt| &rt.feed)),
        operators,
    );
    let static_ids: BTreeSet<&FeedId> = statics.iter().map(|feed| &feed.id).collect();

    let mut by_static: BTreeMap<FeedId, Vec<RealtimeFeed>> = BTreeMap::new();
    for rt in realtime {
        for sibling in operator_feeds.siblings(&rt.feed) {
            if static_ids.contains(&sibling) {
                by_static.entry(sibling).or_default().push(rt.clone());
            }
        }
    }
    by_static
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::dmfr::{AssociatedFeed, RealtimeUrls};

    fn feed(id: &str) -> FeedCore {
        FeedCore {
            id: id.into(),
            operators: vec![],

            authorization: None,
        }
    }

    fn rt_feed(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            feed: feed(id),
            urls: RealtimeUrls::default(),
        }
    }

    fn publishing_trip_updates(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            urls: RealtimeUrls {
                trip_updates: Some("https://example.com/tu.pb".to_owned()),
                ..Default::default()
            },
            ..rt_feed(id)
        }
    }

    fn publishing_alerts(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            urls: RealtimeUrls {
                alerts: Some("https://example.com/a.pb".to_owned()),
                ..Default::default()
            },
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
                    feed_onestop_id: Some((*id).into()),
                })
                .collect(),
        }
    }

    fn join(
        statics: &[FeedCore],
        realtime: &[RealtimeFeed],
    ) -> BTreeMap<FeedId, Vec<RealtimeFeed>> {
        realtime_by_static(statics.iter(), realtime, &[])
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
