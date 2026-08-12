//! Assembling a zone out of the atlas.
//!
//! The schema itself lives in the `transit-zone` crate, which knows nothing
//! about DMFR - that's the point of it being separate, since the deployment
//! reads a zone without an atlas anywhere in sight. This is the other half:
//! turning atlas records into one, which only the picker does.

pub use transit_zone::zone::*;

use crate::dmfr::Feed;
use crate::measure;
use crate::realtime;

use std::collections::{BTreeMap, HashMap};

use geo::Rect;

/// Assembles a zone from matched feeds and what the atlas says about them.
///
/// `feeds` are the selected feeds, in the order they should appear.
/// `realtime` is the operator join from [`crate::realtime::realtime_by_static`],
/// and `credentials` supplies any keys already in hand, keyed by Onestop ID -
/// static and realtime alike.
pub fn assemble<'a>(
    bounds: &Rect,
    feeds: impl IntoIterator<Item = &'a Feed>,
    realtime: &BTreeMap<String, Vec<&'a Feed>>,
    credentials: &HashMap<String, String>,
) -> Zone {
    let feeds = feeds
        .into_iter()
        .map(|feed| ZoneFeed {
            feed_onestop_id: feed.id.clone(),
            provider: feed.display_name(),
            url: feed.urls.static_current.clone().unwrap_or_default(),
            authorization: zone_auth(feed, credentials.get(&feed.id)),
            realtime: realtime
                .get(&feed.id)
                .map(|rts| {
                    rts.iter()
                        .map(|rt| ZoneRealtime {
                            feed_onestop_id: rt.id.clone(),
                            urls: realtime::realtime_urls(rt),
                            authorization: zone_auth(rt, credentials.get(&rt.id)),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();

    Zone {
        version: VERSION,
        bounds: Bounds {
            min_lon: bounds.min().x,
            min_lat: bounds.min().y,
            max_lon: bounds.max().x,
            max_lat: bounds.max().y,
        },
        feeds,
    }
}

fn zone_auth(feed: &Feed, credential: Option<&String>) -> Option<ZoneAuth> {
    let auth = feed.authorization.as_ref()?;
    Some(ZoneAuth {
        kind: auth.kind.clone(),
        param_name: auth.param_name.clone(),
        info_url: auth.info_url.clone(),
        credential: credential.cloned().unwrap_or_default(),
    })
}

/// How to authenticate when fetching a zone's static feed, in the form the
/// downloader wants. The credential itself travels separately, keyed by
/// Onestop ID.
pub fn download_auth(feed: &ZoneFeed) -> Option<measure::Auth> {
    let auth = feed.authorization.as_ref()?;
    Some(measure::Auth {
        kind: auth.kind.clone(),
        param_name: auth
            .param_name
            .clone()
            .filter(|param| !param.trim().is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmfr::{Authorization, Urls};

    use geo::coord;

    fn feed(id: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            spec: "gtfs".to_owned(),
            urls: Urls {
                static_current: Some(format!("https://example.com/{id}.zip")),
                ..Default::default()
            },
            operators: vec![],
            tags: BTreeMap::new(),
            authorization: None,
            source_domain: "example.com".to_owned(),
        }
    }

    #[test]
    fn a_realtime_feeds_endpoints_come_along_with_it() {
        let static_feed = feed("f-c23-kcm");
        let mut rt = feed("f-c23-kcm~rt");
        rt.spec = "gtfs-rt".to_owned();
        rt.urls.realtime_trip_updates = Some("https://example.com/tu.pb".to_owned());
        rt.urls.realtime_alerts = Some("https://example.com/a.pb".to_owned());

        let rects = Rect::new(coord! { x: -122.4, y: 47.4 }, coord! { x: -122.0, y: 47.8 });
        let realtime = BTreeMap::from([("f-c23-kcm".to_owned(), vec![&rt])]);
        let zone = assemble(&rects, [&static_feed], &realtime, &HashMap::new());

        let urls = &zone.feeds[0].realtime[0].urls;
        assert_eq!(
            urls.trip_updates.as_deref(),
            Some("https://example.com/tu.pb")
        );
        assert_eq!(urls.alerts.as_deref(), Some("https://example.com/a.pb"));
        assert_eq!(urls.vehicle_positions, None);
    }

    /// A credential in hand is written into the zone literally: whoever builds
    /// it needs the value to fetch the feed.
    #[test]
    fn a_supplied_credential_lands_on_the_feed() {
        let mut static_feed = feed("f-x");
        static_feed.authorization = Some(Authorization {
            kind: "query_param".to_owned(),
            param_name: Some("api_key".to_owned()),
            info_url: None,
        });

        let credentials = HashMap::from([("f-x".to_owned(), "a-token".to_owned())]);
        let rects = Rect::new(coord! { x: 0.0, y: 0.0 }, coord! { x: 1.0, y: 1.0 });
        let zone = assemble(&rects, [&static_feed], &BTreeMap::new(), &credentials);

        let auth = zone.feeds[0].authorization.as_ref().unwrap();
        assert_eq!(auth.credential, "a-token");
        assert_eq!(download_auth(&zone.feeds[0]).unwrap().kind, "query_param");
    }

    #[test]
    fn an_unauthenticated_feed_has_nothing_for_the_downloader() {
        let rects = Rect::new(coord! { x: 0.0, y: 0.0 }, coord! { x: 1.0, y: 1.0 });
        let zone = assemble(&rects, [&feed("f-x")], &BTreeMap::new(), &HashMap::new());

        assert!(zone.feeds[0].authorization.is_none());
        assert!(download_auth(&zone.feeds[0]).is_none());
    }
}
