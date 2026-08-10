//! Downloads the curated GTFS feeds for a transit zone and repacks them.
//!
//! Reads the CSV that `discover-feeds` produces (and a human then curates),
//! fetches each feed, and writes it back out with a `feed_info.txt` carrying a
//! unique `feed_id`.
//!
//! This replaces `download_gtfs_feeds.py`. Beyond the language, the meaningful
//! change is where the feed identity comes from: the old CSV keyed feeds by
//! MobilityDatabase source id, and the atlas has no such thing, so feeds are
//! now keyed by Onestop ID.

use gtfout::api_keys::ApiKeys;
use gtfout::feed_id::{feed_id_for, rewrite_feed_info};
use gtfout::measure;
use gtfout::Result;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(about = "Download the curated GTFS feeds for a transit zone")]
struct Args {
    /// The zone's curated feed CSV, as produced by discover-feeds.
    #[arg(long)]
    feeds: PathBuf,

    /// Directory to write the repacked feed zips into.
    #[arg(long)]
    output: PathBuf,

    /// Env-file of HEADWAY_GTFS_API_KEY_* lines, for feeds that need a key.
    #[arg(long)]
    api_keys: Option<PathBuf>,
}

/// One row of the curated feed CSV.
///
/// Extra columns are ignored, so the extent columns that discover-feeds emits
/// for human review don't have to be understood here.
#[derive(Debug, Deserialize)]
struct FeedRow {
    feed_onestop_id: String,
    #[serde(default)]
    provider: String,
    url: String,
    #[serde(default)]
    authorization_type: String,
    #[serde(default)]
    authorization_param: String,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    std::fs::create_dir_all(&args.output)?;

    let api_keys = match &args.api_keys {
        Some(path) => ApiKeys::load(path)?,
        None => ApiKeys::from_env(),
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-download-feeds")
        .build()?;

    let mut reader = csv::Reader::from_path(&args.feeds)?;
    let mut count = 0;

    for row in reader.deserialize() {
        let row: FeedRow = row?;
        let feed_id = feed_id_for(&row.feed_onestop_id);

        eprintln!(
            "Downloading {} ({}) from {}",
            feed_id, row.provider, row.url
        );
        let zip_bytes = measure::fetch_feed(
            &client,
            &row.url,
            &row.feed_onestop_id,
            auth_of(&row).as_ref(),
            &api_keys,
        )
        .map_err(|e| {
            format!(
                "could not download GTFS feed for {} ({}): {e}",
                row.provider, row.feed_onestop_id
            )
        })?;

        let output_path = args.output.join(format!("{feed_id}.gtfs.zip"));
        repack(&zip_bytes, &feed_id, &output_path)?;
        count += 1;
    }

    eprintln!("wrote {count} feeds to {}", args.output.display());
    Ok(())
}

fn auth_of(row: &FeedRow) -> Option<measure::Auth> {
    if row.authorization_type.trim().is_empty() {
        return None;
    }
    Some(measure::Auth {
        kind: row.authorization_type.clone(),
        param_name: Some(row.authorization_param.clone()).filter(|param| !param.trim().is_empty()),
    })
}

/// Rewrites the archive with a `feed_info.txt` that names this feed.
fn repack(zip_bytes: &[u8], feed_id: &str, output_path: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let existing_feed_info = archive
        .file_names()
        .find(|name| base_name_is(name, "feed_info.txt"))
        .map(str::to_owned);

    let feed_info = match &existing_feed_info {
        Some(name) => {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut archive.by_name(name)?, &mut contents)?;
            rewrite_feed_info(Some(&contents), feed_id)?
        }
        None => rewrite_feed_info(None, feed_id)?,
    };

    let out = std::fs::File::create(output_path)?;
    let mut writer = zip::ZipWriter::new(out);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        if entry.is_dir() || base_name_is(&name, "feed_info.txt") {
            continue;
        }
        writer.start_file(&name, options)?;
        std::io::copy(&mut entry, &mut writer)?;
    }

    // Written last so it replaces whatever the agency shipped.
    writer.start_file("feed_info.txt", options)?;
    writer.write_all(feed_info.as_bytes())?;
    writer.finish()?;

    Ok(())
}

fn base_name_is(path: &str, name: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|base| base.eq_ignore_ascii_case(name))
}
