use clap::Parser;
use gtfout::atlas::dmfr;
use gtfout::atlas::{self, AtlasSource};
use gtfout::feed_config::{self, FeedConfig};
use gtfout::measure;
use gtfout::transit_zone::router_config::{required_feeds, Scope};
use gtfout::transit_zone::zone::{Zone, ZoneAuth};
use gtfout::Result;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Write each zone the slice of gtfs-secrets.json it needs")]
struct Args {
    build_dir: PathBuf,
    /// Verify credentials against Atlas endpoints without writing zone files.
    #[arg(long)]
    verify: bool,
    /// Existing Atlas clone; defaults to data/transit-zoner/atlas.
    #[arg(long, requires = "verify")]
    atlas_path: Option<PathBuf>,
    /// Clone or refresh the Atlas before verification.
    #[arg(long, requires = "verify")]
    download: bool,
    #[arg(long, default_value_t = atlas::default_repo())]
    atlas_repo: String,
    #[arg(long, default_value_t = atlas::default_ref())]
    atlas_ref: String,
    /// Credentials to write the per-zone files from.
    #[arg(long, default_value = feed_config::DEFAULT_PATH)]
    credentials_file: PathBuf,
}

struct ZoneCredentials {
    path: PathBuf,
    zone: Zone,
    feeds: BTreeSet<String>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    run(Args::parse())
}

fn run(args: Args) -> Result<()> {
    let zones = load_zones(&args.build_dir)?;
    let credentials = FeedConfig::from_file(&args.credentials_file)?;
    let required: BTreeSet<String> = zones
        .iter()
        .flat_map(|zone| zone.feeds.iter().cloned())
        .collect();
    let missing: BTreeSet<String> = required
        .iter()
        .filter(|feed_id| !credentials.has_credential(feed_id))
        .cloned()
        .collect();

    if !args.verify {
        for zone in &zones {
            write_zone_credentials(zone, &credentials)?;
        }
    }
    if !missing.is_empty() {
        print!("{}", missing_template(&zones, &missing));
        return Err(format!(
            "{} of {} credentials are blank; fill them into gtfs-secrets.json and re-run",
            missing.len(),
            required.len()
        )
        .into());
    }
    if !args.verify {
        eprintln!(
            "Wrote credentials for {} feeds in {} zones.",
            required.len(),
            zones.len()
        );
        return Ok(());
    }
    if required.is_empty() {
        eprintln!("No zone needs a credential; nothing to verify.");
        return Ok(());
    }

    let default_path = args.atlas_path.is_none();
    let path = args
        .atlas_path
        .unwrap_or_else(|| PathBuf::from("data/transit-zoner/atlas"));
    let atlas = AtlasSource {
        download: args.download || (default_path && !path.exists()),
        path,
        repo: args.atlas_repo,
        git_ref: args.atlas_ref,
    };
    let catalog = dmfr::load_catalog(atlas.ensure()?)?;
    let probes = verification_endpoints(&catalog, &required)?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-transit-credentials")
        .build()?;
    let mut failures = 0;
    for ((feed_id, url), feed) in probes {
        match measure::verify_feed(&client, feed, &url, &credentials) {
            Ok(()) => println!("✓ {feed_id} {url}"),
            Err(error) => {
                failures += 1;
                eprintln!("✗ {feed_id} {url}: {error}");
            }
        }
    }
    if failures > 0 {
        return Err(format!("{failures} credential endpoints failed").into());
    }
    Ok(())
}

fn load_zones(build_dir: &Path) -> Result<Vec<ZoneCredentials>> {
    let transit = build_dir.join("transit");
    let mut paths = Vec::new();
    for entry in
        fs::read_dir(&transit).map_err(|error| format!("reading {}: {error}", transit.display()))?
    {
        let path = entry?.path().join("zone.json");
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(format!("no zones under {}", transit.display()).into());
    }
    paths
        .into_iter()
        .map(|path| {
            let zone = Zone::load(&path)?;
            let feeds = required_feeds(&zone, Scope::All);
            Ok(ZoneCredentials { path, zone, feeds })
        })
        .collect()
}

fn authenticated_feeds(zone: &Zone) -> impl Iterator<Item = (&str, &ZoneAuth)> {
    zone.feeds
        .iter()
        .flat_map(|feed| {
            std::iter::once((feed.feed_onestop_id.as_str(), feed.authorization.as_ref())).chain(
                feed.realtime
                    .iter()
                    .map(|rt| (rt.feed_onestop_id.as_str(), rt.authorization.as_ref())),
            )
        })
        .filter_map(|(id, auth)| auth.map(|auth| (id, auth)))
}

/// Writes the zone its own `gtfs-secrets.json`, cut down to the feeds this zone uses.
fn write_zone_credentials(zone: &ZoneCredentials, credentials: &FeedConfig) -> Result<()> {
    let path = zone.path.with_file_name(feed_config::DEFAULT_PATH);
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    let subset = credentials.subset(zone.feeds.iter().map(String::as_str));
    writeln!(file, "{}", serde_json::to_string_pretty(&subset)?)?;
    Ok(())
}

fn missing_template(zones: &[ZoneCredentials], missing: &BTreeSet<String>) -> String {
    let mut entries = BTreeMap::new();
    for zone in zones {
        for (id, auth) in authenticated_feeds(&zone.zone) {
            if !missing.contains(id) {
                continue;
            }
            let mut entry = format!(
                "  {{\"feed_id\": {id:?}, \"key\": \"\"}}   // {}",
                auth.kind
            );
            if let Some(param) = &auth.param_name {
                entry.push_str(&format!(" {param:?}"));
            }
            if let Some(url) = &auth.info_url {
                entry.push_str(&format!(", request one at {url}"));
            }
            entries.insert(id.to_owned(), entry);
        }
    }
    entries.into_values().collect::<Vec<_>>().join("\n")
}

fn verification_endpoints<'a>(
    catalog: &'a dmfr::Catalog,
    required: &BTreeSet<String>,
) -> Result<BTreeMap<(String, String), &'a dmfr::Feed>> {
    let mut probes = BTreeMap::new();
    let mut found = BTreeSet::new();
    let mut add = |feed: &'a dmfr::Feed, url: Option<&str>| {
        if feed.authorization.is_none() || !required.contains(&feed.id) {
            return;
        }
        if let Some(url) = url.filter(|url| !url.trim().is_empty()) {
            found.insert(feed.id.clone());
            probes.insert((feed.id.clone(), url.to_owned()), feed);
        }
    };
    for feed in &catalog.static_feeds {
        add(&feed.feed, feed.url.as_deref());
        for realtime in &feed.realtime {
            for url in [
                &realtime.alerts_url,
                &realtime.trip_updates_url,
                &realtime.vehicle_positions_url,
            ] {
                add(&realtime.feed, url.as_deref());
            }
        }
    }
    let unmatched: Vec<_> = required.difference(&found).cloned().collect();
    if !unmatched.is_empty() {
        return Err(format!(
            "no authenticated Atlas endpoint for {}",
            unmatched.join(", ")
        )
        .into());
    }
    Ok(probes)
}
