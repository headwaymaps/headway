//! The zone file - what the transit-zoner writes and the build reads - and
//! assembling one out of the atlas.
//!
//! Everything but the conversion from the atlas was its own `transit-zone`
//! crate, until it turned
//! out gtfout was the only thing that ever depended on it.

pub mod feed_info;
pub mod router_config;
pub mod zone;

pub use zone::*;

use crate::atlas::dmfr::AuthKind;
use crate::atlas::dmfr::StaticFeed;

use geo::Rect;

/// Everything the zone file records about one atlas feed.
///
/// `build-gtfs-index` converts each feed once and stores the result in the
/// index, which is where transit-zoner reads it back from.
impl From<&StaticFeed> for ZoneFeed {
    fn from(gtfs: &StaticFeed) -> Self {
        ZoneFeed {
            feed_onestop_id: gtfs.id().to_owned(),
            provider: gtfs.display_name(),
            url: gtfs.url.clone(),
            authorization: gtfs.feed.authorization.clone(),
            realtime: gtfs
                .realtime
                .iter()
                .map(|rt| ZoneRealtime {
                    feed_onestop_id: rt.id().to_owned(),
                    urls: rt.urls.clone(),
                    authorization: rt.feed.authorization.clone(),
                })
                .collect(),
        }
    }
}

impl Zone {
    /// Assembles a zone from the described feeds a person picked off the map.
    pub fn new(bounds: &Rect, feeds: Vec<ZoneFeed>) -> Self {
        Self {
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
}

impl ZoneFeed {
    /// The auth this feed's schedule download needs, if any.
    pub fn auth_kind(&self) -> Option<&AuthKind> {
        Some(&self.authorization.as_ref()?.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::dmfr::{
        AuthKind, Authorization, FeedCore, FeedId, RealtimeFeed, RealtimeUrls,
    };

    use geo::coord;

    fn core(id: &str) -> FeedCore {
        FeedCore {
            id: id.into(),
            operators: vec![],
            authorization: None,
        }
    }

    fn feed(id: &str) -> StaticFeed {
        StaticFeed {
            feed: core(id),
            url: format!("https://example.com/{id}.zip"),
            realtime: vec![],
        }
    }

    fn trip_updates(id: &str) -> RealtimeFeed {
        RealtimeFeed {
            feed: core(id),
            urls: RealtimeUrls {
                trip_updates: Some("https://example.com/tu.pb".to_owned()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn what_the_transit_zoner_writes_is_what_the_build_reads() {
        let mut static_feed = feed("f-c23-kcm");
        static_feed.feed.authorization = Some(Authorization {
            kind: AuthKind::QueryParam {
                param_name: "api_key".to_owned(),
            },
            info_url: Some("https://example.com/keys".to_owned()),
        });
        static_feed.realtime.push(trip_updates("f-c23-kcm~rt"));

        let bounds = Rect::new(coord! { x: -122.4, y: 47.4 }, coord! { x: -122.0, y: 47.8 });
        let zone = Zone::new(&bounds, vec![ZoneFeed::from(&static_feed)]);

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
                .kind
                .param_name(),
            Some("api_key")
        );
        assert_eq!(reloaded_feed.realtime.len(), 1);

        let (config, skipped) = reloaded.router_config(&crate::feed_config::FeedConfig::default());
        assert!(skipped.is_empty());
        assert_eq!(config.updaters.len(), 1);
        assert_eq!(config.updaters[0].feed_id, FeedId::from("f-c23-kcm"));
        assert_eq!(config.updaters[0].url, "https://example.com/tu.pb");
    }
}
