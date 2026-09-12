//! The zone file - what the transit-zoner writes and the build reads - and
//! assembling one out of the atlas.
//!
//! Everything but `assemble` was its own `transit-zone` crate, until it turned
//! out gtfout was the only thing that ever depended on it.

pub mod feed_id;
pub mod router_config;
pub mod zone;

pub use zone::*;

use crate::atlas::dmfr::{Feed, RealtimeFeed, StaticFeed};
use crate::measure;

use geo::Rect;

/// Everything the zone file records about one atlas feed.
///
/// `build-gtfs-index` calls this once per feed and stores the result in the
/// index, which is where transit-zoner reads it back from.
pub fn describe(gtfs: &StaticFeed) -> ZoneFeed {
    ZoneFeed {
        feed_onestop_id: gtfs.id().to_owned(),
        provider: gtfs.display_name(),
        url: gtfs.url.clone().unwrap_or_default(),
        authorization: zone_auth(&gtfs.feed),
        realtime: gtfs
            .realtime
            .iter()
            .map(|rt| ZoneRealtime {
                feed_onestop_id: rt.id().to_owned(),
                urls: realtime_urls(rt),
                authorization: zone_auth(&rt.feed),
            })
            .collect(),
    }
}

/// Assembles a zone from the described feeds a person picked off the map.
pub fn assemble(bounds: &Rect, feeds: Vec<ZoneFeed>) -> Zone {
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

/// The endpoints a realtime feed publishes, as a zone records them.
fn realtime_urls(rt: &RealtimeFeed) -> RealtimeUrls {
    RealtimeUrls {
        trip_updates: rt.trip_updates_url.clone(),
        vehicle_positions: rt.vehicle_positions_url.clone(),
        alerts: rt.alerts_url.clone(),
    }
}

fn zone_auth(feed: &Feed) -> Option<ZoneAuth> {
    let auth = feed.authorization.as_ref()?;
    Some(ZoneAuth {
        kind: auth.kind.clone(),
        param_name: auth.param_name.clone(),
        info_url: auth.info_url.clone(),
    })
}

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
    use crate::atlas::dmfr::{Authorization, RealtimeFeed};

    use geo::coord;

    fn core(id: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            operators: vec![],
            authorization: None,
        }
    }

    fn feed(id: &str) -> StaticFeed {
        StaticFeed {
            feed: core(id),
            url: Some(format!("https://example.com/{id}.zip")),
            realtime: vec![],
        }
    }

    fn trip_updates(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            feed: core(id),
            alerts_url: None,
            trip_updates_url: Some("https://example.com/tu.pb".to_owned()),
            vehicle_positions_url: None,
        }
    }

    #[test]
    fn what_the_transit_zoner_writes_is_what_the_build_reads() {
        let mut static_feed = feed("f-c23-kcm");
        static_feed.feed.authorization = Some(Authorization {
            kind: "query_param".to_owned(),
            param_name: Some("api_key".to_owned()),
            info_url: Some("https://example.com/keys".to_owned()),
        });
        static_feed.realtime.push(trip_updates("f-c23-kcm~rt"));

        let bounds = Rect::new(coord! { x: -122.4, y: 47.4 }, coord! { x: -122.0, y: 47.8 });
        let zone = assemble(&bounds, vec![describe(&static_feed)]);

        let document = serde_json::to_string_pretty(&zone).unwrap();
        let reloaded =
            Zone::parse(&document).expect("the build must accept what transit-zoner wrote");

        assert_eq!(reloaded.version, VERSION);
        assert_eq!(reloaded.bounds.min_lon, -122.4);
        assert_eq!(reloaded.feeds.len(), 1);
        let reloaded_feed = &reloaded.feeds[0];
        assert_eq!(reloaded_feed.feed_onestop_id, "f-c23-kcm");
        assert_eq!(reloaded_feed.url, "https://example.com/f-c23-kcm.zip");
        assert_eq!(
            reloaded_feed
                .authorization
                .as_ref()
                .unwrap()
                .param_name
                .as_deref(),
            Some("api_key")
        );
        assert_eq!(reloaded_feed.realtime.len(), 1);

        let (config, skipped) = reloaded.router_config(&crate::feed_config::FeedConfig::default());
        assert!(skipped.is_empty());
        assert_eq!(config.updaters.len(), 1);
        assert_eq!(
            config.updaters[0].feed_id,
            super::feed_id::feed_id_for("f-c23-kcm")
        );
        assert_eq!(config.updaters[0].url, "https://example.com/tu.pb");
    }
}
