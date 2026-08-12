//! Measures where every GTFS feed in the Transitland Atlas actually is.
//!
//! The atlas ships no geometry at all - DMFR has no bounding box and no country
//! field - so the only reliable way to know where a feed serves is to download
//! it and read its stops. This binary does that for the whole catalog and
//! records the result in a GeoPackage, which the zone builder then queries per
//! area.
//!
//! # Why the whole catalog
//!
//! An earlier version measured only the feeds surviving a bbox-relative
//! prefilter, into the same GeoPackage that queries then read as though it were
//! complete - so a zone's answer depended on which zones had been built before
//! it. Measuring unconditionally makes "what has been measured" a property of
//! the index rather than of your run history.
//!
//! It costs one full download pass, once: feeds already in the index are never
//! re-fetched, so the index is worth keeping in a cache volume.

use gtfout::atlas::{self, AtlasSource};
use gtfout::dmfr::{self, Feed};
use gtfout::extents::FeedExtents;
use gtfout::feed_config::{self, FeedConfig};
use gtfout::measure::{self, Measurement};
use gtfout::progress::{format_bytes, Progress};
use gtfout::Result;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use clap::{ArgGroup, Parser};

#[derive(Parser, Debug)]
#[command(about = "Measure GTFS feeds from a transitland-atlas clone into a spatial index")]
// Scope is required rather than defaulted. --all is a multi-GB download pass,
// which is not something to start by forgetting a flag.
#[command(group(ArgGroup::new("scope").required(true).args(["all", "feeds"])))]
struct Args {
    /// Measure every GTFS feed in the atlas.
    ///
    /// The expensive one - thousands of downloads - but feeds already measured
    /// are skipped, so it's a one-off.
    #[arg(long)]
    all: bool,

    /// Measure only this feed, by Onestop ID. Repeatable.
    ///
    /// Named feeds are always re-measured, which makes this the way to retry one
    /// feed after supplying its credential, without --retry-failed's catalog-wide
    /// sweep. An unknown ID is an error, since a typo would otherwise look
    /// exactly like a feed with nothing to do.
    #[arg(long = "feed", value_name = "ONESTOP_ID")]
    feeds: Vec<String>,

    /// Path to a transitland-atlas clone.
    ///
    /// Read-only unless --download is given, in which case this is where the
    /// clone is created or refreshed.
    #[arg(long)]
    atlas_path: PathBuf,

    /// Clone the atlas into --atlas-path if it isn't there, or refresh it if it
    /// is. Idempotent, so re-running keeps the catalog current.
    ///
    /// Without this the path is never written to and a missing one is an error -
    /// which is the point: it stops a mistyped --atlas-path from silently
    /// becoming a fresh clone.
    #[arg(long)]
    download: bool,

    /// Where --download fetches from. Defaults to $HEADWAY_TRANSITLAND_ATLAS_URL,
    /// else upstream.
    #[arg(long, default_value_t = atlas::default_repo())]
    atlas_repo: String,

    /// Which ref --download tracks. Defaults to $HEADWAY_TRANSITLAND_ATLAS_REF,
    /// else "main".
    #[arg(long, default_value_t = atlas::default_ref())]
    atlas_ref: String,

    /// GeoPackage to write, created if absent.
    ///
    /// A local build artifact: derived data, rebuildable by re-downloading, and
    /// not something to commit. Keeping it between runs is what makes the first
    /// pass a one-off rather than a recurring cost.
    #[arg(long, default_value = "./feed-extents.gpkg")]
    out: PathBuf,

    /// YAML config of credentials for feeds that need one to be fetched.
    ///
    /// A `feeds:` table keyed by Onestop ID. Without it those feeds are recorded
    /// as failures; see --write-config-template.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Write a config template here, listing every feed the atlas says needs a
    /// credential that we still have no extent for.
    ///
    /// Merges with an existing file at this path rather than overwriting it, so
    /// regenerating never costs you credentials you've already collected.
    #[arg(long)]
    write_config_template: Option<PathBuf>,

    /// Re-measure feeds that previously failed.
    ///
    /// Failures are remembered so a dead server isn't retried every run - which
    /// also means adding a missing API key has no effect until you pass this.
    #[arg(long)]
    retry_failed: bool,

    /// How many feeds to download at once.
    #[arg(long, default_value_t = 8)]
    concurrency: usize,

    /// Report how much work a run would do and stop, without downloading.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    let config = match &args.config {
        Some(path) => FeedConfig::load(path)?,
        None => FeedConfig::default(),
    };
    if let Some(path) = &args.config {
        if config.is_empty() {
            eprintln!(
                "warning: {} has no credentials filled in - authenticated feeds will fail",
                path.display()
            );
        }
    }

    let atlas = AtlasSource {
        path: args.atlas_path,
        download: args.download,
        repo: args.atlas_repo,
        git_ref: args.atlas_ref,
    };
    // Kept whole: --write-config-template needs the authorization block and
    // operator name of feeds that failed.
    let catalog = dmfr::load_catalog(atlas.ensure()?)?;

    let selected = select_feeds(&catalog, &args.feeds)?;

    // Only `static_current` is worth fetching. `static_historic` is a
    // provenance trail of superseded URLs and is frequently dead.
    let mut to_measure: Vec<(Feed, String)> = Vec::new();
    let mut no_url = Vec::new();
    for feed in selected {
        match feed.urls.static_current.clone() {
            Some(url) if !url.trim().is_empty() => to_measure.push((feed.clone(), url)),
            _ => no_url.push(feed.id.clone()),
        }
    }
    if args.all {
        eprintln!(
            "{} gtfs feeds with a static_current url, {} without",
            to_measure.len(),
            no_url.len()
        );
    } else {
        eprintln!("{} feed(s) named", to_measure.len() + no_url.len());
        // Silent under --all, where it's 42 feeds nobody asked about.
        for feed_id in &no_url {
            eprintln!("  {feed_id} has no static_current url, so there's nothing to fetch");
        }
    }

    let extents = FeedExtents::open(&args.out)?;

    if args.retry_failed {
        let forgotten = extents.forget_failures()?;
        eprintln!("--retry-failed: forgot {forgotten} previously failed measurements");
    }

    let candidates = to_measure.len();
    let outstanding: Vec<(Feed, String)> = if args.feeds.is_empty() {
        // Anything already in the GeoPackage stays there - that's the point of
        // it, and what makes a second --all run nearly free.
        let already_measured = extents.measured_keys()?;
        to_measure
            .into_iter()
            .filter(|(feed, url)| !already_measured.contains_key(&(feed.id.clone(), url.clone())))
            .collect()
    } else {
        // Naming a feed means "measure this one now". Skipping it because the
        // index already had an answer would make the flag do nothing at all;
        // insert() replaces the old record rather than duplicating it.
        to_measure
    };
    eprintln!(
        "{} already measured, {} to fetch",
        candidates - outstanding.len(),
        outstanding.len()
    );

    if args.dry_run {
        eprintln!("--dry-run: stopping before downloading anything");
        // Still useful here - the template comes from what the index already
        // knows, so this answers "what credentials am I missing" for free.
        if let Some(path) = &args.write_config_template {
            write_config_template(path, &catalog, &extents)?;
        }
        return Ok(());
    }

    measure_all(outstanding, &extents, args.concurrency, &config)?;

    let index_size = std::fs::metadata(&args.out).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "\nindex written to {} ({})",
        args.out.display(),
        format_bytes(index_size)
    );

    if let Some(path) = &args.write_config_template {
        write_config_template(path, &catalog, &extents)?;
    }

    // Whole-index totals, not just this run's: a feed that failed three runs ago
    // is still missing from every query today. Individual failures already
    // printed as they happened.
    let failures = extents.failures()?;
    for (feed_id, url, error) in &failures {
        log::debug!("failed {feed_id} ({url}): {error}");
    }
    if !failures.is_empty() {
        eprintln!(
            "{} feeds in the index have no extent and so match no area. \
             Re-run with RUST_LOG=debug to list them, or --retry-failed to try again.",
            failures.len()
        );
    }

    Ok(())
}

/// The feeds this run covers: the whole GTFS catalog, or just the named ones.
/// An empty `wanted` means `--all`, which clap has already guaranteed.
///
/// Unknown names are reported all at once rather than one run at a time.
fn select_feeds<'a>(catalog: &'a dmfr::Catalog, wanted: &[String]) -> Result<Vec<&'a Feed>> {
    let gtfs: Vec<&Feed> = catalog.feeds.iter().filter(|f| f.is_gtfs()).collect();

    if wanted.is_empty() {
        return Ok(gtfs);
    }

    let by_id: HashMap<&str, &Feed> = gtfs.iter().map(|feed| (feed.id.as_str(), *feed)).collect();

    let mut selected = Vec::new();
    let mut unknown = Vec::new();
    for id in wanted {
        match by_id.get(id.as_str()) {
            Some(feed) => selected.push(*feed),
            None => unknown.push(id.as_str()),
        }
    }

    if !unknown.is_empty() {
        // Naming a gtfs-rt feed is the likely mistake - the atlas is full of
        // them and their IDs look just like these - so distinguish "no such
        // feed" from "wrong kind of feed". One line, because main prints errors
        // with Debug and a newline would come out escaped.
        let detail: Vec<String> = unknown
            .iter()
            .map(|id| match catalog.feeds.iter().find(|f| f.id == *id) {
                Some(other) => format!("{id} (in the atlas, but its spec is {})", other.spec),
                None => format!("{id} (no such feed)"),
            })
            .collect();
        return Err(format!("not a GTFS feed in this atlas: {}", detail.join("; ")).into());
    }

    Ok(selected)
}

/// Writes the config template of feeds still waiting on a credential.
///
/// "Needs one" is the atlas's own claim - the feed carries an `authorization`
/// block - intersected with "we have no extent for it". That pairing keeps the
/// list honest both ways: a feed that measured fine needs nothing from you, and
/// a 404 isn't something a credential fixes.
///
/// Read from the index rather than this run's results, so it still lists
/// everything after a re-run that skipped the failures.
fn write_config_template(
    path: &std::path::Path,
    catalog: &dmfr::Catalog,
    extents: &FeedExtents,
) -> Result<()> {
    let failed: std::collections::HashSet<String> = extents
        .failures()?
        .into_iter()
        .map(|(feed_id, _, _)| feed_id)
        .collect();

    let needing: Vec<&Feed> = catalog
        .feeds
        .iter()
        .filter(|feed| feed.authorization.is_some() && failed.contains(&feed.id))
        .collect();

    // Merge rather than clobber: this path usually already holds credentials
    // someone spent time collecting.
    let existing = if path.exists() {
        FeedConfig::load(path).map_err(|e| {
            format!(
                "{} exists but isn't a feed config, so it won't be overwritten: {e}",
                path.display()
            )
        })?
    } else {
        FeedConfig::default()
    };

    std::fs::write(path, feed_config::template(&needing, &existing))?;

    eprintln!(
        "wrote a config template for {} feed(s) needing a credential to {}",
        needing.len(),
        path.display()
    );
    if !needing.is_empty() {
        eprintln!(
            "Fill it in, then re-run with --config {} --retry-failed",
            path.display()
        );
    }
    Ok(())
}

/// Downloads and measures feeds in parallel, writing results to the GeoPackage.
///
/// Downloads fan out across threads, but SQLite takes a single writer, so
/// results funnel back through a channel and are written in batches. Batching
/// also means an interrupted run keeps everything up to the last batch.
fn measure_all(
    to_measure: Vec<(Feed, String)>,
    extents: &FeedExtents,
    concurrency: usize,
    config: &FeedConfig,
) -> Result<()> {
    let total = to_measure.len();
    if total == 0 {
        return Ok(());
    }

    let queue = Mutex::new(to_measure.into_iter());
    let progress = Progress::new(total);
    let (tx, rx) = mpsc::channel::<(String, String, Measurement)>();

    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-write-gtfs-index")
        .build()?;

    // The GeoPackage's SQLite connection isn't Sync, so it can't cross into a
    // worker thread. Writing therefore happens here on the main thread while
    // the workers download.
    std::thread::scope(|scope| -> Result<()> {
        for _ in 0..concurrency.max(1) {
            let tx = tx.clone();
            let client = &client;
            let queue = &queue;
            let progress = &progress;
            scope.spawn(move || loop {
                let Some((feed, url)) = queue.lock().unwrap().next() else {
                    break;
                };

                let outcome = measure::measure(client, &feed, &url, config);
                progress.record(&feed.id, &outcome);

                if tx.send((feed.id, url, outcome.measurement)).is_err() {
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

        progress.finish();
        Ok(())
    })
}

/// Rows per GeoPackage transaction.
const WRITE_BATCH: usize = 64;
