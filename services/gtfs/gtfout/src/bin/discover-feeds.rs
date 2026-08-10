//! Finds the GTFS feeds serving an area, from a transitland-atlas clone.
//!
//! This replaces the old MobilityDatabase flow. The MobilityDatabase ships a
//! bounding box per feed, so discovery there was a CSV filter. The atlas ships
//! no geometry at all, so we work it out ourselves:
//!
//! 1. Read every feed record from the clone.
//! 2. Drop the ones we have positive evidence are somewhere else entirely
//!    (see [`gtfout::prefilter`]) - about 60% of the catalog for a typical
//!    metro area, which is what keeps step 3 affordable.
//! 3. Download whatever is left and measure where its stops actually are.
//! 4. Emit the feeds whose extent intersects the area.
//!
//! The output is a starting point for a human, not a final answer: it lands in
//! `builds/<config>/transit/gtfs-feeds/<zone>.gtfs_feeds.csv` and is meant to
//! be reviewed and trimmed before building a transit zone.

use gtfout::api_keys::ApiKeys;
use gtfout::dmfr::{self, Feed};
use gtfout::extents::FeedExtents;
use gtfout::geom::{Point, Rect};
use gtfout::measure::{self, Measurement};
use gtfout::prefilter::{Prefilter, Skip};
use gtfout::realtime::{self, Associations, RouterConfig};
use gtfout::Result;

use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Discover GTFS feeds serving an area from a transitland-atlas clone")]
struct Args {
    /// Path to a transitland-atlas clone (https://github.com/transitland/transitland-atlas)
    #[arg(long)]
    atlas: PathBuf,

    /// Area of interest, as "<min_lon> <min_lat> <max_lon> <max_lat>"
    ///
    /// `allow_hyphen_values` because western-hemisphere longitudes are
    /// negative, and clap would otherwise read "-122.462 ..." as a flag.
    #[arg(long, allow_hyphen_values = true)]
    bbox: String,

    /// GeoPackage of measured feed extents, created if absent.
    ///
    /// A local build artifact: derived data, rebuildable by re-downloading, and
    /// not something to commit. Keeping it between runs is what makes a second
    /// transit zone cost a spatial query rather than another download pass.
    #[arg(long, default_value = "./feed-extents.gpkg")]
    extents: PathBuf,

    /// Country codes that stay in scope, e.g. "us,ca". Be generous: a feed
    /// excluded here never reaches the output.
    #[arg(long, value_delimiter = ',', default_value = "")]
    countries: Vec<String>,

    /// US state / Canadian province codes that stay in scope, e.g. "wa,or,bc".
    #[arg(long, value_delimiter = ',', default_value = "")]
    regions: Vec<String>,

    /// Measure every feed, skipping the prefilter entirely. Slow - thousands of
    /// downloads - but it answers "why isn't agency X in my output?" without
    /// having to trust the exclusion rules.
    #[arg(long)]
    no_prefilter: bool,

    /// Report what the prefilter would do and stop, without downloading.
    #[arg(long)]
    dry_run: bool,

    /// How many feeds to download at once.
    #[arg(long, default_value_t = 8)]
    concurrency: usize,

    /// Write the CSV here instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Also write an OTP router-config.json of GTFS-RT updaters for the
    /// discovered feeds.
    #[arg(long)]
    router_config: Option<PathBuf>,

    /// Env-file of HEADWAY_GTFS_API_KEY_* lines, for feeds that need a key to
    /// be fetched. Without it those feeds can't be measured.
    #[arg(long)]
    api_keys: Option<PathBuf>,
}

fn parse_bbox(s: &str) -> Result<Rect> {
    let values: Vec<f64> = s
        .split_whitespace()
        .map(|v| v.parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| format!("invalid bbox {s:?}: {e}"))?;

    let [min_lon, min_lat, max_lon, max_lat] = values[..] else {
        return Err(format!(
            "bbox needs 4 values (<min_lon> <min_lat> <max_lon> <max_lat>), got {}",
            values.len()
        )
        .into());
    };

    Ok(Rect::new(
        Point::new(min_lon, min_lat),
        Point::new(max_lon, max_lat),
    ))
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    let area = parse_bbox(&args.bbox)?;

    let api_keys = match &args.api_keys {
        Some(path) => ApiKeys::load(path)?,
        None => ApiKeys::from_env(),
    };

    let catalog = dmfr::load_catalog(&args.atlas)?;
    let gtfs: Vec<Feed> = catalog
        .feeds
        .iter()
        .filter(|f| f.is_gtfs())
        .cloned()
        .collect();
    eprintln!("{} gtfs feeds in the atlas", gtfs.len());

    let prefilter = Prefilter::new(area.clone(), args.countries.clone(), args.regions.clone());

    let mut to_measure: Vec<(Feed, String)> = Vec::new();
    let mut skipped: Vec<(String, Skip)> = Vec::new();
    let mut no_url = 0;

    for feed in gtfs {
        if !args.no_prefilter {
            if let Some(reason) = prefilter.skip_reason(&feed) {
                skipped.push((feed.id.clone(), reason));
                continue;
            }
        }
        // Only `static_current` is worth fetching. `static_historic` is a
        // provenance trail of superseded URLs and is frequently dead.
        match feed.urls.static_current.clone() {
            Some(url) if !url.trim().is_empty() => to_measure.push((feed, url)),
            _ => no_url += 1,
        }
    }

    report_skips(&skipped);
    eprintln!(
        "{} feeds to measure, {} skipped by the prefilter, {} with no static_current url",
        to_measure.len(),
        skipped.len(),
        no_url
    );

    if args.dry_run {
        eprintln!("--dry-run: stopping before downloading anything");
        return Ok(());
    }

    let extents = FeedExtents::open(&args.extents)?;

    // Anything already in the GeoPackage stays there - that's the point of it.
    let already_measured = extents.measured_keys()?;
    let outstanding: Vec<(Feed, String)> = to_measure
        .iter()
        .filter(|(feed, url)| !already_measured.contains_key(&(feed.id.clone(), url.clone())))
        .cloned()
        .collect();
    eprintln!(
        "{} already measured, {} to fetch",
        to_measure.len() - outstanding.len(),
        outstanding.len()
    );

    measure_all(outstanding, &extents, args.concurrency, &api_keys)?;

    // The output is driven by an indexed spatial query over everything ever
    // measured, not just this run's downloads.
    let matching_ids = extents.feeds_in(&area)?;
    let by_id: HashMap<&str, &(Feed, String)> = to_measure
        .iter()
        .map(|entry| (entry.0.id.as_str(), entry))
        .collect();

    let mut matched: Vec<&(Feed, String)> = matching_ids
        .iter()
        // A feed can be in the GeoPackage from an earlier run for a different
        // area while being out of scope for this one - measured, but filtered
        // out before we got here. It isn't a candidate now.
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .collect();
    matched.sort_by(|a, b| a.0.id.cmp(&b.0.id));

    let mut out: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(std::fs::File::create(path)?),
        None => Box::new(std::io::stdout()),
    };
    write_csv(&mut out, &matched, &extents)?;

    eprintln!("\n{} feeds intersect the area", matched.len());

    if let Some(path) = &args.router_config {
        write_router_config(path, &catalog, &matched)?;
    }

    // A feed we couldn't fetch is not a feed we know to be elsewhere. Reporting
    // it beats dropping it silently, since the point of this output is that a
    // human reviews it.
    let in_scope: std::collections::HashSet<&str> = to_measure
        .iter()
        .map(|(feed, _)| feed.id.as_str())
        .collect();
    let failures: Vec<_> = extents
        .failures()?
        .into_iter()
        .filter(|(feed_id, _, _)| in_scope.contains(feed_id.as_str()))
        .collect();

    if !failures.is_empty() {
        eprintln!(
            "{} feeds could not be measured and are NOT in the output:",
            failures.len()
        );
        for (feed_id, url, error) in &failures {
            eprintln!("  {feed_id} ({url}): {error}");
        }
    }

    Ok(())
}

/// Summarize exclusions by reason, so an over-eager rule is visible.
fn report_skips(skipped: &[(String, Skip)]) {
    let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
    for (_, skip) in skipped {
        let label = match skip {
            Skip::GeohashFarAway { .. } => "geohash far away",
            Skip::ForeignAggregator { .. } => "foreign aggregator tag",
            Skip::ForeignTld { .. } => "foreign ccTLD",
            Skip::DistantRegion { .. } => "distant state/province",
        };
        *counts.entry(label).or_default() += 1;
    }
    for (label, count) in counts {
        eprintln!("  skipped {count:5} : {label}");
    }
    // Individual reasons at debug level: too noisy for normal runs, but the
    // first thing you want when a feed you expected isn't in the output.
    for (id, skip) in skipped {
        log::debug!("skipped {id}: {skip}");
    }
}

/// Downloads and measures feeds in parallel, writing results to the GeoPackage.
///
/// Downloads fan out across threads, but SQLite takes a single writer, so the
/// results funnel back through a channel and are written in batches by one
/// thread. Batching also means an interrupted run keeps everything up to the
/// last batch instead of losing the lot.
fn measure_all(
    to_measure: Vec<(Feed, String)>,
    extents: &FeedExtents,
    concurrency: usize,
    api_keys: &ApiKeys,
) -> Result<()> {
    let total = to_measure.len();
    if total == 0 {
        return Ok(());
    }

    let queue = Mutex::new(to_measure.into_iter());
    let done = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<(String, String, Measurement)>();

    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-discover-feeds")
        .build()?;

    // The GeoPackage's SQLite connection isn't Sync, so it can't cross into a
    // worker thread. Writing therefore happens here on the main thread while
    // the workers download.
    std::thread::scope(|scope| -> Result<()> {
        for _ in 0..concurrency.max(1) {
            let tx = tx.clone();
            let client = &client;
            let queue = &queue;
            let done = &done;
            scope.spawn(move || loop {
                let Some((feed, url)) = queue.lock().unwrap().next() else {
                    break;
                };

                let measurement = measure::measure(client, &feed, &url, api_keys);

                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(50) || n == total {
                    eprintln!("  measured {n}/{total}");
                }

                if tx.send((feed.id, url, measurement)).is_err() {
                    break;
                }
            });
        }

        // Drop our own sender, or the loop below never sees the channel close.
        drop(tx);

        let mut batch = Vec::with_capacity(WRITE_BATCH);
        for measurement in rx {
            batch.push(measurement);
            if batch.len() >= WRITE_BATCH {
                extents.insert(&batch)?;
                batch.clear();
            }
        }
        if !batch.is_empty() {
            extents.insert(&batch)?;
        }

        Ok(())
    })
}

/// Rows per GeoPackage transaction.
const WRITE_BATCH: usize = 64;

/// Writes the OTP `router-config.json` of realtime updaters for the feeds we
/// discovered.
///
/// Emitted alongside the CSV rather than derived from it later, because the
/// association from an RT feed to its static feed lives in the atlas, and this
/// is where the atlas is already in hand. Regenerate it after curating the CSV
/// to drop updaters for feeds you removed.
fn write_router_config(
    path: &std::path::Path,
    catalog: &dmfr::Catalog,
    matched: &[&(Feed, String)],
) -> Result<()> {
    let associations = Associations::build(&catalog.feeds, &catalog.operators);
    let static_ids: BTreeSet<String> = matched.iter().map(|(feed, _)| feed.id.clone()).collect();
    let (updaters, skipped) = realtime::updaters_for(&catalog.feeds, &static_ids, &associations);

    let json = serde_json::to_string_pretty(&RouterConfig { updaters })?;
    std::fs::write(path, json + "\n")?;

    eprintln!("wrote realtime updaters to {}", path.display());
    for skip in &skipped {
        eprintln!("  skipped realtime feed {}: {}", skip.feed_id, skip.reason);
    }
    Ok(())
}

/// The curated-feed CSV consumed by the rest of the transit build.
///
/// The extent columns aren't used downstream - they're there for whoever
/// reviews this file. A feed spanning half a continent (Amtrak, FlixBus) really
/// does intersect your metro area, and seeing its size is what lets a curator
/// decide whether they want it in the zone.
fn write_csv(
    out: &mut dyn Write,
    candidates: &[&(Feed, String)],
    extents: &FeedExtents,
) -> Result<()> {
    let mut writer = csv::Writer::from_writer(out);
    writer.write_record([
        "feed_onestop_id",
        "provider",
        "url",
        "authorization_type",
        "authorization_param",
        "min_lon",
        "min_lat",
        "max_lon",
        "max_lat",
    ])?;

    let boxes = extents.extents_by_feed_id()?;

    for (feed, url) in candidates {
        let Some(bbox) = boxes.get(&feed.id) else {
            continue;
        };
        let (auth_type, auth_param) = match &feed.authorization {
            Some(auth) => (
                auth.kind.clone(),
                auth.param_name.clone().unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };

        writer.write_record([
            &feed.id,
            &feed.display_name(),
            url,
            &auth_type,
            &auth_param,
            &bbox.min().x().to_string(),
            &bbox.min().y().to_string(),
            &bbox.max().x().to_string(),
            &bbox.max().y().to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bbox() {
        let bbox = parse_bbox("-122.462 47.394 -122.005 47.831").unwrap();
        assert_eq!(bbox.min(), Point::new(-122.462, 47.394));
        assert_eq!(bbox.max(), Point::new(-122.005, 47.831));
    }

    #[test]
    fn rejects_a_malformed_bbox() {
        assert!(parse_bbox("1 2 3").is_err());
        assert!(parse_bbox("1 2 3 4 5").is_err());
        assert!(parse_bbox("north south east west").is_err());
    }
}
