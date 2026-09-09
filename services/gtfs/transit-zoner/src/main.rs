//! An interactive builder for a transit zone's feed list.

use geo::{coord, Rect};
use gtfout::extents::FeedExtents;
use gtfout::geom::RectExt;
use gtfout::transit_zone::assemble;
use gtfout::transit_zone::zone::{ZoneFeed, ZoneRealtime};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use actix_web::http::StatusCode;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(about = "Build a transit zone's GTFS feeds from a map")]
struct Args {
    /// GeoPackage of measured feed extents, from `build-gtfs-index`.
    #[arg(long)]
    gtfs_index: PathBuf,

    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value_t = 8420)]
    port: u16,
}

/// A realtime feed riding along with a static one.
#[derive(Debug, Clone, Serialize)]
struct RealtimeSummary {
    feed_id: String,
    /// Which streams it publishes, e.g. "trip updates", "vehicle positions".
    kinds: Vec<&'static str>,
    authorization_type: String,
    /// Where to request a credential, when the atlas says.
    info_url: Option<String>,
}

/// One feed as the page sees it.
#[derive(Debug, Clone, Serialize)]
struct FeedSummary {
    feed_id: String,
    provider: String,
    url: String,
    authorization_type: String,
    info_url: Option<String>,
    /// Realtime feeds updating this one. They have no extent of their own, so
    /// this is the only way they reach a zone.
    realtime: Vec<RealtimeSummary>,
    /// [min_lon, min_lat, max_lon, max_lat], for drawing it on the map.
    bbox: [f64; 4],
    area_m2: f64,
    relevance: Option<f64>,
}

/// A static feed transit-zoner can offer: one the index has measured *and*
/// described from the atlas.
struct MeasuredFeed {
    feed: ZoneFeed,
    extent: Rect,
}

struct State {
    /// SQLite's connection isn't Sync, so it can't be shared across actix
    /// workers unguarded.
    extents: Mutex<FeedExtents>,
    /// The offerable feeds, by id. Read once at startup: rebuilding the index
    /// needs a restart anyway.
    feeds: HashMap<String, MeasuredFeed>,
}

fn realtime_summary(rt: &ZoneRealtime) -> RealtimeSummary {
    RealtimeSummary {
        feed_id: rt.feed_onestop_id.clone(),
        kinds: stream_kinds(rt),
        authorization_type: rt
            .authorization
            .as_ref()
            .map(|a| a.kind.clone())
            .unwrap_or_default(),
        info_url: rt.authorization.as_ref().and_then(|a| a.info_url.clone()),
    }
}

/// Which streams a realtime feed publishes, for the page to label it.
fn stream_kinds(rt: &ZoneRealtime) -> Vec<&'static str> {
    [
        (&rt.urls.trip_updates, "trip updates"),
        (&rt.urls.vehicle_positions, "vehicle positions"),
        (&rt.urls.alerts, "alerts"),
    ]
    .into_iter()
    .filter_map(|(url, label)| url.as_ref().map(|_| label))
    .collect()
}

/// An error the page can show a person as-is.
///
/// Saying `text/plain` is what lets the page tell our message apart from the
/// HTML error page a proxy serves when this service is down; without it the
/// page has no way to know it's holding a gateway error rather than advice.
fn text_error(status: StatusCode, message: impl Into<String>) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("text/plain; charset=utf-8")
        .body(message.into())
}

#[derive(Debug, Deserialize)]
struct BboxQuery {
    /// "min_lon,min_lat,max_lon,max_lat", as the page builds it.
    bbox: String,
}

fn parse_bbox(raw: &str) -> Option<Rect> {
    let values: Vec<f64> = raw
        .split(',')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<Vec<f64>>>()?;
    let [min_lon, min_lat, max_lon, max_lat] = values[..] else {
        return None;
    };
    if !(-180.0..=180.0).contains(&min_lon)
        || !(-180.0..=180.0).contains(&max_lon)
        || !(-90.0..=90.0).contains(&min_lat)
        || !(-90.0..=90.0).contains(&max_lat)
        || min_lon >= max_lon
        || min_lat >= max_lat
    {
        return None;
    }
    Some(Rect::new(
        coord! { x: min_lon, y: min_lat },
        coord! { x: max_lon, y: max_lat },
    ))
}

/// Describes the named feeds, in the order they were named.
fn summarize<'a>(
    state: &State,
    ids: impl IntoIterator<Item = &'a String>,
    area: Option<&Rect>,
) -> Vec<FeedSummary> {
    ids.into_iter()
        .filter_map(|id| {
            let measured = state.feeds.get(id)?;
            let feed = &measured.feed;
            let bbox = &measured.extent;
            Some(FeedSummary {
                feed_id: feed.feed_onestop_id.clone(),
                provider: feed.provider.clone(),
                url: feed.url.clone(),
                authorization_type: feed
                    .authorization
                    .as_ref()
                    .map(|a| a.kind.clone())
                    .unwrap_or_default(),
                info_url: feed.authorization.as_ref().and_then(|a| a.info_url.clone()),
                realtime: feed.realtime.iter().map(realtime_summary).collect(),
                bbox: [bbox.min().x, bbox.min().y, bbox.max().x, bbox.max().y],
                area_m2: bbox.area_m2(),
                relevance: area.map(|area| area.jaccard(bbox)),
            })
        })
        .collect()
}

/// The feeds intersecting a box, best match first.
async fn list_feeds(state: web::Data<State>, query: web::Query<BboxQuery>) -> impl Responder {
    let Some(area) = parse_bbox(&query.bbox) else {
        return text_error(
            StatusCode::BAD_REQUEST,
            "bbox must be min_lon,min_lat,max_lon,max_lat",
        );
    };

    let matching = {
        let extents = state.extents.lock().unwrap();
        match extents.feeds_in(&area) {
            Ok(matching) => matching,
            Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    };

    let mut summaries = summarize(&state, &matching, Some(&area));
    summaries.sort_by(|a, b| {
        b.relevance
            .unwrap_or_default()
            .total_cmp(&a.relevance.unwrap_or_default())
            .then_with(|| a.feed_id.cmp(&b.feed_id))
    });

    HttpResponse::Ok().json(summaries)
}

/// The named feeds, whether or not they intersect anything.
async fn feeds_by_id(state: web::Data<State>, path: web::Path<String>) -> impl Responder {
    let ids: Vec<String> = path
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();

    HttpResponse::Ok().json(summarize(&state, &ids, None))
}

#[derive(Debug, Deserialize)]
struct ZoneRequest {
    bbox: String,
    feed_ids: Vec<String>,
}

/// The zone document: area, feeds and realtime in one file.
async fn download_zone(state: web::Data<State>, request: web::Json<ZoneRequest>) -> impl Responder {
    let Some(area) = parse_bbox(&request.bbox) else {
        return text_error(
            StatusCode::BAD_REQUEST,
            "bbox must be min_lon,min_lat,max_lon,max_lat",
        );
    };

    let mut feeds = Vec::with_capacity(request.feed_ids.len());
    for id in &request.feed_ids {
        let Some(measured) = state.feeds.get(id) else {
            return text_error(
                StatusCode::BAD_REQUEST,
                format!("{id} is not offerable: not measured in the index, or no longer in the atlas when it was built"),
            );
        };
        feeds.push(measured.feed.clone());
    }

    let zone = assemble(&area, feeds);

    match serde_json::to_string_pretty(&zone) {
        Ok(json) => HttpResponse::Ok()
            .content_type("application/json; charset=utf-8")
            .insert_header(("content-disposition", "attachment; filename=\"zone.json\""))
            .body(json + "\n"),
        Err(e) => text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// All the setup happens before the server binds, so handling a request at all
/// means we're ready.
async fn health_ready() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if !args.gtfs_index.exists() {
        eprintln!(
            "no index at {} - build one with `build-gtfs-index --out {}`",
            args.gtfs_index.display(),
            args.gtfs_index.display()
        );
        std::process::exit(1);
    }

    let extents = FeedExtents::open(&args.gtfs_index).unwrap_or_else(|e| {
        eprintln!("opening {}: {e}", args.gtfs_index.display());
        std::process::exit(1);
    });

    let feeds: HashMap<String, MeasuredFeed> = extents
        .measured_feeds()
        .unwrap_or_else(|e| {
            eprintln!("reading {}: {e}", args.gtfs_index.display());
            std::process::exit(1);
        })
        .into_iter()
        .map(|(feed, extent)| (feed.feed_onestop_id.clone(), MeasuredFeed { feed, extent }))
        .collect();

    if feeds.is_empty() {
        eprintln!(
            "{} describes no measured feeds - re-run build-gtfs-index over it",
            args.gtfs_index.display()
        );
        std::process::exit(1);
    }

    let with_realtime = feeds
        .values()
        .filter(|m| !m.feed.realtime.is_empty())
        .count();
    eprintln!(
        "{} measured gtfs feeds, {with_realtime} of them with realtime",
        feeds.len()
    );

    let state = web::Data::new(State {
        extents: Mutex::new(extents),
        feeds,
    });

    eprintln!("transit-zoner on http://{}:{}", args.host, args.port);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(web::JsonConfig::default().limit(4 * 1024 * 1024))
            .route("/health/ready", web::get().to(health_ready))
            .route("/api/feeds-by-bbox", web::get().to(list_feeds))
            .route("/api/feeds/{ids}", web::get().to(feeds_by_id))
            .route("/api/zone", web::post().to(download_zone))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_bbox_the_page_sends() {
        let bbox = parse_bbox("-122.462,47.394,-122.005,47.831").unwrap();
        assert_eq!(bbox.min(), coord! { x: -122.462, y: 47.394 });
        assert_eq!(bbox.max(), coord! { x: -122.005, y: 47.831 });
    }

    #[test]
    fn rejects_invalid_geographic_bounds() {
        for bbox in [
            "NaN,0,1,1",
            "0,0,inf,1",
            "-181,0,1,1",
            "0,-91,1,1",
            "2,0,1,1",
            "0,2,1,1",
            "0,0,0,1",
        ] {
            assert!(parse_bbox(bbox).is_none(), "{bbox}");
        }
    }

    /// The page keys off this to tell our errors from a proxy's error page.
    #[test]
    fn an_error_says_it_is_plain_text() {
        let response = text_error(StatusCode::BAD_REQUEST, "bbox must be ...");
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn rejects_a_malformed_bbox() {
        assert!(parse_bbox("1,2,3").is_none());
        assert!(parse_bbox("1,2,3,4,5").is_none());
        assert!(parse_bbox("").is_none());
        assert!(parse_bbox("north,south,east,west").is_none());
        assert!(parse_bbox("-122.4,47.4,junk,-122.0,47.8").is_none());
    }
}
