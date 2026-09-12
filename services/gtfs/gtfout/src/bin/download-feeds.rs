//! Downloads the curated GTFS feeds for a transit zone and repacks them.

use gtfout::feed_config::{self, FeedConfig};
use gtfout::measure;
use gtfout::transit_zone::feed_id::{feed_id_for, rewrite_feed_info};
use gtfout::transit_zone::{download_auth, Zone, ZoneFeed};
use gtfout::Result;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Download the curated GTFS feeds for a transit zone")]
struct Args {
    /// The zone file, as produced by transit-zoner.
    #[arg(long)]
    zone: PathBuf,

    /// Directory to write the repacked feed zips into.
    #[arg(long)]
    output: PathBuf,

    /// Credentials for the token-gated feeds this zone uses.
    #[arg(long, default_value = feed_config::DEFAULT_PATH)]
    credentials_file: PathBuf,
}

/// One feed to fetch.
#[derive(Debug, PartialEq)]
struct FeedToDownload {
    feed_onestop_id: String,
    provider: String,
    url: String,
    auth: Option<measure::Auth>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    std::fs::create_dir_all(&args.output)?;

    let credentials = FeedConfig::from_file(&args.credentials_file)?;

    let zone = Zone::load(&args.zone)?;

    let unfilled = feeds_missing_credentials(&zone, &credentials);
    if !unfilled.is_empty() {
        return Err(missing_credentials_error(&unfilled).into());
    }

    let feeds = from_zone(&zone);

    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-download-feeds")
        .build()?;

    for feed in &feeds {
        let feed_id = feed_id_for(&feed.feed_onestop_id);

        eprintln!(
            "Downloading {} ({}) from {}",
            feed_id, feed.provider, feed.url
        );
        let zip_bytes = measure::fetch_feed(
            &client,
            &feed.url,
            &feed.feed_onestop_id,
            feed.auth.as_ref(),
            &credentials,
        )
        .map_err(|e| {
            format!(
                "could not download GTFS feed for {} ({}): {e}",
                feed.provider, feed.feed_onestop_id
            )
        })?;

        let output_path = args.output.join(format!("{feed_id}.gtfs.zip"));
        repack(&zip_bytes, &feed_id, &output_path)?;
    }

    eprintln!("wrote {} feeds to {}", feeds.len(), args.output.display());
    Ok(())
}

fn from_zone(zone: &Zone) -> Vec<FeedToDownload> {
    zone.feeds
        .iter()
        .map(|feed| FeedToDownload {
            feed_onestop_id: feed.feed_onestop_id.clone(),
            provider: feed.provider.clone(),
            url: feed.url.clone(),
            auth: download_auth(feed),
        })
        .collect()
}

/// Feeds the zone says need a credential, paired with where to ask for one.
fn feeds_missing_credentials<'a>(zone: &'a Zone, credentials: &FeedConfig) -> Vec<&'a ZoneFeed> {
    zone.feeds
        .iter()
        .filter(|feed| {
            feed.authorization.is_some() && credentials.get(&feed.feed_onestop_id).is_none()
        })
        .collect()
}

/// Names the feeds with no secret, and where each token comes from.
fn missing_credentials_error(unfilled: &[&ZoneFeed]) -> String {
    let mut message = format!(
        "no credential for {}. Add an entry for {} to gtfs-secrets.json \
         (bin/transit-credentials writes the blanks for you):",
        unfilled
            .iter()
            .map(|feed| feed.feed_onestop_id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        if unfilled.len() == 1 { "it" } else { "them" },
    );
    for feed in unfilled {
        message.push_str(&format!(
            "\n  {{\"feed_id\": {:?}, \"key\": \"\"}}",
            feed.feed_onestop_id
        ));
        if let Some(info_url) = feed
            .authorization
            .as_ref()
            .and_then(|auth| auth.info_url.as_deref())
        {
            message.push_str(&format!("   # request one at: {info_url}"));
        }
    }
    message
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

#[cfg(test)]
mod tests {
    use super::*;

    const ZONE_JSON: &str = r#"{
      "version": 1,
      "bounds": { "min_lon": -122.5, "min_lat": 47.3, "max_lon": -122.0, "max_lat": 47.8 },
      "feeds": [
        {
          "feed_onestop_id": "f-c23-kingcountymetro",
          "provider": "King County Metro",
          "url": "https://example.com/kcm.zip"
        },
        {
          "feed_onestop_id": "f-9q8y-sfmta",
          "provider": "SFMTA",
          "url": "https://example.com/sf.zip",
          "authorization": { "type": "query_param", "param_name": "api_key",
                             "info_url": "https://example.com/keys" }
        }
      ]
    }"#;

    fn zone() -> Zone {
        Zone::parse(ZONE_JSON).unwrap()
    }

    fn config(pairs: &[(&str, &str)]) -> FeedConfig {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn an_unsupplied_credential_is_caught_before_anything_is_fetched() {
        let zone = zone();

        let missing = feeds_missing_credentials(&zone, &FeedConfig::default());
        assert_eq!(
            missing
                .iter()
                .map(|f| f.feed_onestop_id.as_str())
                .collect::<Vec<_>>(),
            ["f-9q8y-sfmta"]
        );

        let supplied = config(&[("f-9q8y-sfmta", "a-token")]);
        assert!(feeds_missing_credentials(&zone, &supplied).is_empty());
    }

    #[test]
    fn the_error_names_the_variable_and_where_to_ask() {
        let zone = zone();
        let message =
            missing_credentials_error(&feeds_missing_credentials(&zone, &FeedConfig::default()));

        assert!(
            message.contains(r#""feed_id": "f-9q8y-sfmta""#),
            "{message}"
        );
        assert!(message.contains("https://example.com/keys"), "{message}");
    }

    #[test]
    fn a_blank_value_does_not_count_as_supplied() {
        let zone = zone();
        let blank = config(&[("f-9q8y-sfmta", "  ")]);
        assert_eq!(feeds_missing_credentials(&zone, &blank).len(), 1);
    }
}
