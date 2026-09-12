//! Measures where every GTFS feed in the Transitland Atlas actually is, into
//! the index the transit-zoner serves.

mod progress;

use gtfout::atlas::dmfr::{self, Feed, StaticFeed};
use gtfout::atlas::{self, AtlasSource};
use gtfout::extents::FeedExtents;
use gtfout::feed_config::{self, FeedConfig};
use gtfout::measure::{self, Measurement};
use gtfout::transit_zone::{self, zone::ZoneFeed};
use gtfout::Result;
use progress::{format_bytes, Progress};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use clap::Parser;

/// Where the index and its atlas clone live, relative to the repo root.
const DEFAULT_ATLAS_PATH: &str = "data/transit-zoner/atlas";
const DEFAULT_INDEX_PATH: &str = "data/transit-zoner/feed-extents.gpkg";

const LONG_ABOUT: &str = "\
Builds and updates the GTFS feed-extents index - the GeoPackage that says which
feeds cover which area.

Building it from scratch is slow (~30m; every catalogued feed gets downloaded
and measured), so the index lives on disk and is updated in place: feeds already
measured successfully are skipped and everything else is fetched, including
feeds that failed on an earlier run. That is how tokens added to
gtfs-secrets.json reach the index.

The transit-zoner dev server reads the index from disk. The transit-zoner image
does not - it downloads the copy published in headway-data - so publishing an
update means committing it there as gtfs/feed-extents.gpkg and pushing.";

const EXAMPLES: &str = "\
Examples:
  build-gtfs-index                        update the index
  build-gtfs-index --dry-run              report what an update would fetch
  build-gtfs-index --feed f-9q8y-sfmta    re-measure one feed
  build-gtfs-index --dry-run --write-config-template gtfs-secrets.json";

#[derive(Parser, Debug)]
#[command(about = "Build and update the GTFS feed-extents index")]
#[command(long_about = LONG_ABOUT, after_help = EXAMPLES)]
struct Args {
    /// Measure only this feed, by Onestop ID. Repeatable. Naming none measures
    /// every GTFS feed in the atlas.
    #[arg(long = "feed", value_name = "ONESTOP_ID")]
    feeds: Vec<String>,

    /// GeoPackage to create or update.
    #[arg(long, default_value = DEFAULT_INDEX_PATH)]
    out: PathBuf,

    /// Path to a transitland-atlas clone, cloned if it isn't there.
    #[arg(long, default_value = DEFAULT_ATLAS_PATH)]
    atlas_path: PathBuf,

    /// Read the atlas at --atlas-path as it stands, without refreshing it.
    #[arg(long)]
    no_download: bool,

    /// Where the atlas is fetched from. Defaults to
    /// $HEADWAY_TRANSITLAND_ATLAS_URL, else upstream.
    #[arg(long, default_value_t = atlas::default_repo())]
    atlas_repo: String,

    /// Which atlas ref to track. Defaults to $HEADWAY_TRANSITLAND_ATLAS_REF,
    /// else "main".
    #[arg(long, default_value_t = atlas::default_ref())]
    atlas_ref: String,

    /// Credentials for token-gated feeds. Feeds needing one we don't hold are
    /// measured anyway, and fail.
    #[arg(long, default_value = feed_config::DEFAULT_PATH)]
    credentials_file: PathBuf,

    /// Write a credentials template here, listing every feed the atlas says
    /// needs a credential that we still have no extent for. Writes a whole
    /// file, so point it somewhere other than a filled-in credentials file.
    #[arg(long)]
    write_config_template: Option<PathBuf>,

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

    let config = FeedConfig::from_file(&args.credentials_file)?;
    if config.is_empty() {
        let path = args.credentials_file.display();
        eprintln!("warning: no credentials in {path}, so token-gated feeds will fail to measure.");
        eprintln!("Generate a template and fill in your tokens:");
        eprintln!("    build-gtfs-index --dry-run --write-config-template {path}");
        eprintln!("Re-running later picks up feeds as their tokens arrive.");
    }

    if let Some(parent) = args.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }

    let atlas = AtlasSource {
        path: args.atlas_path,
        download: !args.no_download,
        repo: args.atlas_repo,
        git_ref: args.atlas_ref,
    };
    let catalog = dmfr::load_catalog(atlas.ensure()?)?;

    let selected = select_feeds(&catalog, &args.feeds)?;

    let mut to_measure: Vec<(Feed, String)> = Vec::new();
    let mut no_url = Vec::new();
    for feed in selected {
        match feed.url.clone() {
            Some(url) if !url.trim().is_empty() => to_measure.push((feed.feed.clone(), url)),
            _ => no_url.push(feed.id().to_owned()),
        }
    }
    if args.feeds.is_empty() {
        eprintln!(
            "{} gtfs feeds with a static_current url, {} without",
            to_measure.len(),
            no_url.len()
        );
    } else {
        eprintln!("{} feed(s) named", to_measure.len() + no_url.len());
        for feed_id in &no_url {
            eprintln!("  {feed_id} has no static_current url, so there's nothing to fetch");
        }
    }

    let extents = FeedExtents::open(&args.out)?;

    let candidates = to_measure.len();
    let mut retries = 0;
    let outstanding: Vec<(Feed, String)> = if args.feeds.is_empty() {
        let already_measured = extents.measured_keys()?;
        to_measure
            .into_iter()
            .filter(
                |(feed, url)| match already_measured.get(&(feed.id.clone(), url.clone())) {
                    None => true,
                    Some(true) => false,
                    Some(false) => {
                        retries += 1;
                        true
                    }
                },
            )
            .collect()
    } else {
        to_measure
    };
    eprintln!(
        "{} already measured, {} to fetch",
        candidates - outstanding.len(),
        outstanding.len()
    );
    if retries > 0 {
        eprintln!("{retries} of those failed on an earlier run");
    }

    // What the atlas says about each feed, refreshed on every run: a feed whose
    // DMFR record changed keeps its measurement but must not keep the old url,
    // provider or authorization. Nothing here is downloaded, so a dry run
    // refreshes an existing index too.
    let describe_measured = || -> Result<()> {
        let described: Vec<(String, ZoneFeed)> = catalog
            .static_feeds
            .iter()
            .map(|feed| (feed.id().to_owned(), transit_zone::describe(feed)))
            .collect();
        eprintln!(
            "described {} measured feeds from the atlas",
            extents.set_metadata(&described)?
        );
        Ok(())
    };

    if args.dry_run {
        describe_measured()?;
        eprintln!("--dry-run: stopping before downloading anything");
        if let Some(path) = &args.write_config_template {
            write_config_template(path, &catalog, &extents)?;
        }
        return Ok(());
    }

    measure_all(outstanding, &extents, args.concurrency, &config)?;
    describe_measured()?;

    let index_size = std::fs::metadata(&args.out).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "\nindex written to {} ({})",
        args.out.display(),
        format_bytes(index_size)
    );

    if let Some(path) = &args.write_config_template {
        write_config_template(path, &catalog, &extents)?;
    }

    let failures = extents.failures()?;
    for (feed_id, url, error) in &failures {
        log::debug!("failed {feed_id} ({url}): {error}");
    }
    if !failures.is_empty() {
        eprintln!(
            "{} feeds in the index have no extent and so match no area. \
             Re-run with RUST_LOG=debug to list them.",
            failures.len()
        );
    }

    Ok(())
}

/// The feeds this run covers: the whole GTFS catalog, or just the named ones.
/// Naming none is how you ask for all of them.
fn select_feeds<'a>(catalog: &'a dmfr::Catalog, wanted: &[String]) -> Result<Vec<&'a StaticFeed>> {
    if wanted.is_empty() {
        return Ok(catalog.static_feeds.iter().collect());
    }

    let by_id: HashMap<&str, &StaticFeed> = catalog
        .static_feeds
        .iter()
        .map(|feed| (feed.id(), feed))
        .collect();

    let mut selected = Vec::new();
    let mut unknown = Vec::new();
    for id in wanted {
        match by_id.get(id.as_str()) {
            Some(feed) => selected.push(*feed),
            None => unknown.push(id.as_str()),
        }
    }

    if !unknown.is_empty() {
        let detail: Vec<String> = unknown
            .iter()
            .map(|id| match catalog.unsupported_spec(id) {
                Some(spec) => format!("{id} (in the atlas, but its spec is {spec})"),
                None => format!("{id} (no such feed)"),
            })
            .collect();
        return Err(format!("not a GTFS feed in this atlas: {}", detail.join("; ")).into());
    }

    Ok(selected)
}

/// Writes the config template of feeds still waiting on a credential.
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
        .static_feeds
        .iter()
        .map(|gtfs| &gtfs.feed)
        .filter(|feed| feed.authorization.is_some() && failed.contains(&feed.id))
        .collect();

    std::fs::write(path, feed_config::template(&needing)?)?;

    eprintln!(
        "wrote a credentials template for {} feed(s) needing a credential to {}",
        needing.len(),
        path.display()
    );
    if !needing.is_empty() {
        // gtfs-secrets.json holds no comments, so what each feed wants is reported here.
        eprint!("{}", feed_config::template_guidance(&needing));
        eprintln!("Merge it into gtfs-secrets.json, fill in the keys, then re-run to measure them");
    }
    Ok(())
}

/// Downloads and measures feeds in parallel, writing results to the GeoPackage.
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
        .user_agent("headway-build-gtfs-index")
        .build()?;

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
