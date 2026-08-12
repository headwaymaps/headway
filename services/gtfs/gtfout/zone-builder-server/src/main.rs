//! An interactive builder for a transit zone's feed list.
//!
//! Curating a zone isn't a query you run once: you nudge the area, drop a
//! regional rail feed spanning three states, notice a neighbouring county's bus
//! feed you do want, repeat. So it's a map with the index behind it rather than
//! a flag on a command.
//!
//! This is the only thing that writes a zone file - the document the build and
//! the deployment both read. See [`gtfout::zone`].
//!
//! The index has to exist already: this only reads it, and never touches the
//! network. Build one with `write-gtfs-index --all`.

use geo::{coord, Rect};
use gtfout::dmfr::{self, Feed};
use gtfout::extents::FeedExtents;
use gtfout::geom::RectExt;
use gtfout::realtime::{self, Associations};
use gtfout::zone::assemble;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(about = "Build a transit zone's GTFS feeds from a map")]
struct Args {
    /// GeoPackage of measured feed extents, from `write-gtfs-index`.
    ///
    /// Must already exist - a feed missing from the index is invisible here.
    #[arg(long)]
    gtfs_index: PathBuf,

    /// Path to a transitland-atlas clone. The index holds geometry; the provider
    /// name, URL and authorization that go in the zone file only exist in DMFR.
    #[arg(long)]
    atlas_path: PathBuf,

    /// MapLibre style URL for the basemap. Defaults to the public maps.earth
    /// tileserver, the same style the frontend uses, so this works with nothing
    /// else running; point it at a local one to avoid the round trip.
    #[arg(long, default_value = "https://maps.earth/tileserver/style/basic-v2")]
    map_style: String,

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
    /// How much ground the feed covers, in square meters. It's what makes a
    /// continent-spanning feed obvious next to a city one, and it's measured
    /// rather than in degrees so feeds at different latitudes compare.
    area_m2: f64,
    /// How well the feed's extent matches the drawn area, 0 to 1, and the order
    /// the list is in. Absent when nothing was drawn to compare against - the
    /// by-id lookup, which is asked about feeds wherever they are.
    relevance: Option<f64>,
}

struct State {
    /// SQLite's connection isn't Sync, so it can't be shared across actix
    /// workers unguarded. Queries are millisecond RTree reads, so one lock is
    /// cheaper than reopening the file per request.
    extents: Mutex<FeedExtents>,
    /// The atlas is immutable once loaded, so it needs no lock.
    feeds: HashMap<String, Feed>,
    /// Precomputed operator join, keyed by static feed id.
    realtime: HashMap<String, Vec<RealtimeSummary>>,
    /// Every feed including realtime, for resolving an RT feed's authorization.
    feeds_all: HashMap<String, Feed>,
    map_style: String,
}

fn realtime_summary(feed: &Feed) -> RealtimeSummary {
    RealtimeSummary {
        feed_id: feed.id.clone(),
        // Same labels the zone file uses, so the tags on a row and the document
        // it produces don't disagree.
        kinds: realtime::stream_kinds(feed),
        authorization_type: feed
            .authorization
            .as_ref()
            .map(|a| a.kind.clone())
            .unwrap_or_default(),
        info_url: feed.authorization.as_ref().and_then(|a| a.info_url.clone()),
    }
}

#[derive(Debug, Deserialize)]
struct BboxQuery {
    /// "min_lon,min_lat,max_lon,max_lat", as the page builds it.
    bbox: String,
}

fn parse_bbox(raw: &str) -> Option<Rect> {
    let values: Vec<f64> = raw
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let [min_lon, min_lat, max_lon, max_lat] = values[..] else {
        return None;
    };
    Some(Rect::new(
        coord! { x: min_lon, y: min_lat },
        coord! { x: max_lon, y: max_lat },
    ))
}

/// The page, with the configured basemap substituted in - the one thing it
/// can't work out for itself.
async fn index(state: web::Data<State>) -> impl Responder {
    const PAGE: &str = include_str!("../assets/zone-builder-server/index.html");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(PAGE.replace("__MAP_STYLE__", &state.map_style))
}

/// Describes the named feeds, in the order they were named.
///
/// Ordering is the caller's business: a query has an area to rank against, a
/// lookup by id has whatever order the caller asked in and no reason to disturb
/// it. Feeds the index or the atlas doesn't have are dropped, so the result can
/// be shorter than `ids`.
///
/// The geometry comes from the index rather than from whatever named these
/// feeds, so a page reopening a saved zone is looking at today's measurements.
fn summarize<'a>(
    state: &State,
    ids: impl IntoIterator<Item = &'a String>,
    boxes: &HashMap<String, Rect>,
    area: Option<&Rect>,
) -> Vec<FeedSummary> {
    ids.into_iter()
        .filter_map(|id| {
            // In the index but gone from the atlas: no URL, so not a candidate.
            let feed = state.feeds.get(id)?;
            let bbox = boxes.get(id)?;
            Some(FeedSummary {
                feed_id: feed.id.clone(),
                provider: feed.display_name(),
                url: feed.urls.static_current.clone().unwrap_or_default(),
                authorization_type: feed
                    .authorization
                    .as_ref()
                    .map(|a| a.kind.clone())
                    .unwrap_or_default(),
                info_url: feed.authorization.as_ref().and_then(|a| a.info_url.clone()),
                realtime: state.realtime.get(id).cloned().unwrap_or_default(),
                bbox: [bbox.min().x, bbox.min().y, bbox.max().x, bbox.max().y],
                area_m2: bbox.area_m2(),
                relevance: area.map(|area| area.jaccard(bbox)),
            })
        })
        .collect()
}

/// The feeds intersecting a box, best match first.
///
/// Relevance rather than size, because size alone puts Amtrak above the city
/// bus: what belongs at the top is the operator whose service area looks like
/// the zone being drawn. See [`RectExt::jaccard`].
async fn list_feeds(state: web::Data<State>, query: web::Query<BboxQuery>) -> impl Responder {
    let Some(area) = parse_bbox(&query.bbox) else {
        return HttpResponse::BadRequest().body("bbox must be min_lon,min_lat,max_lon,max_lat");
    };

    let (matching, boxes) = {
        let extents = state.extents.lock().unwrap();
        match (extents.feeds_in(&area), extents.extents_by_feed_id()) {
            (Ok(matching), Ok(boxes)) => (matching, boxes),
            (Err(e), _) | (_, Err(e)) => {
                return HttpResponse::InternalServerError().body(e.to_string())
            }
        }
    };

    let mut summaries = summarize(&state, &matching, &boxes, Some(&area));
    summaries.sort_by(|a, b| {
        b.relevance
            .unwrap_or_default()
            .total_cmp(&a.relevance.unwrap_or_default())
            .then_with(|| a.feed_id.cmp(&b.feed_id))
    });

    HttpResponse::Ok().json(summaries)
}

/// The named feeds, whether or not they intersect anything.
///
/// This is how reopening a zone gets back the feeds its box no longer covers.
/// Asking by id rather than by some box big enough to contain them keeps that
/// exact - and keeps a zone holding one continental feed from having to query
/// the continent to recover a county bus feed.
///
/// Several ids come comma-separated in the one segment: a zone asks about its
/// whole feed list at once, and Onestop IDs carry nothing that needs escaping.
/// They come back in the order asked for - the caller has an order in mind, and
/// with no drawn area there's nothing here that knows better.
async fn feeds_by_id(state: web::Data<State>, path: web::Path<String>) -> impl Responder {
    let ids: Vec<String> = path
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();

    let boxes = {
        let extents = state.extents.lock().unwrap();
        match extents.extents_by_feed_id() {
            Ok(boxes) => boxes,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        }
    };

    HttpResponse::Ok().json(summarize(&state, &ids, &boxes, None))
}

#[derive(Debug, Deserialize)]
struct ZoneRequest {
    bbox: String,
    feed_ids: Vec<String>,
    /// Feed id to credential, for the feeds that need one.
    #[serde(default)]
    credentials: HashMap<String, String>,
}

/// The zone document: area, feeds, realtime and credentials in one file.
///
/// Assembled here rather than in the page so the schema has one definition -
/// [`gtfout::zone`] - which the build system can read back.
async fn download_zone(state: web::Data<State>, request: web::Json<ZoneRequest>) -> impl Responder {
    let Some(area) = parse_bbox(&request.bbox) else {
        return HttpResponse::BadRequest().body("bbox must be min_lon,min_lat,max_lon,max_lat");
    };

    let mut feeds = Vec::with_capacity(request.feed_ids.len());
    let mut realtime: BTreeMap<String, Vec<&Feed>> = BTreeMap::new();
    for id in &request.feed_ids {
        let Some(feed) = state.feeds.get(id) else {
            // A zone naming a feed the atlas doesn't have would build wrong.
            return HttpResponse::BadRequest().body(format!("{id} is not in the atlas"));
        };
        feeds.push(feed);

        // State keeps the association as summaries, for the page; the zone
        // wants the feeds themselves, which feeds_all still has.
        if let Some(summaries) = state.realtime.get(id) {
            let rts: Vec<&Feed> = summaries
                .iter()
                .filter_map(|rt| state.feeds_all.get(&rt.feed_id))
                .collect();
            realtime.insert(id.clone(), rts);
        }
    }

    let zone = assemble(&area, feeds, &realtime, &request.credentials);

    match serde_json::to_string_pretty(&zone) {
        Ok(json) => HttpResponse::Ok()
            .content_type("application/json; charset=utf-8")
            .insert_header(("content-disposition", "attachment; filename=\"zone.json\""))
            .body(json + "\n"),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    if !args.gtfs_index.exists() {
        eprintln!(
            "no index at {} - build one with `write-gtfs-index --all --out {}`",
            args.gtfs_index.display(),
            args.gtfs_index.display()
        );
        std::process::exit(1);
    }

    let extents = FeedExtents::open(&args.gtfs_index).unwrap_or_else(|e| {
        eprintln!("opening {}: {e}", args.gtfs_index.display());
        std::process::exit(1);
    });

    let catalog = dmfr::load_catalog(&args.atlas_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let associations = Associations::build(&catalog.feeds, &catalog.operators);
    let realtime: HashMap<String, Vec<RealtimeSummary>> =
        realtime::realtime_by_static(&catalog.feeds, &associations)
            .into_iter()
            .map(|(static_id, rt)| (static_id, rt.into_iter().map(realtime_summary).collect()))
            .collect();

    let feeds: HashMap<String, Feed> = catalog
        .feeds
        .iter()
        .filter(|f| f.is_gtfs())
        .map(|feed| (feed.id.clone(), feed.clone()))
        .collect();
    let feeds_all: HashMap<String, Feed> = catalog
        .feeds
        .iter()
        .map(|feed| (feed.id.clone(), feed.clone()))
        .collect();
    eprintln!(
        "{} gtfs feeds in the atlas, {} of them with realtime",
        feeds.len(),
        realtime.len()
    );

    let state = web::Data::new(State {
        extents: Mutex::new(extents),
        feeds,
        realtime,
        feeds_all,
        map_style: args.map_style.clone(),
    });

    eprintln!("zone builder server on http://{}:{}", args.host, args.port);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            // 4000 IDs of ~40 chars overruns actix's 32KB default, failing as a
            // confusing 400 exactly when someone selects a whole region.
            .app_data(web::JsonConfig::default().limit(4 * 1024 * 1024))
            .route("/", web::get().to(index))
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
    fn rejects_a_malformed_bbox() {
        assert!(parse_bbox("1,2,3").is_none());
        assert!(parse_bbox("1,2,3,4,5").is_none());
        assert!(parse_bbox("").is_none());
        assert!(parse_bbox("north,south,east,west").is_none());
    }
}
