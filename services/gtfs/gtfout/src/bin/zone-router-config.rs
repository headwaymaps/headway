//! Renders a zone's OTP `router-config.json` on stdout.
//!
//! The rendered config holds live GTFS-RT credentials, so this runs in the OTP
//! init container against the mounted `gtfs-secrets.json` rather than at build
//! time into a committed manifest.

use clap::Parser;
use gtfout::feed_config::{self, FeedConfig};
use gtfout::transit_zone::router_config::Scope;
use gtfout::transit_zone::zone::Zone;
use gtfout::Result;

use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Render a zone's OTP router-config.json")]
struct Args {
    /// The zone file, as produced by transit-zoner.
    #[arg(long)]
    zone: PathBuf,

    /// Credentials for the token-gated feeds this zone uses.
    #[arg(long, default_value = feed_config::DEFAULT_PATH)]
    credentials_file: PathBuf,

    /// Instead of rendering, name the feeds whose credentials this zone needs.
    #[arg(long, value_enum)]
    required_feeds: Option<Scope>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    let zone = Zone::load(&args.zone)?;

    if let Some(which) = args.required_feeds {
        for feed_id in zone.required_feeds(which) {
            println!("{feed_id}");
        }
        return Ok(());
    }

    let credentials = FeedConfig::from_file(&args.credentials_file)?;
    let (router_config, skipped) = zone.router_config(&credentials);

    for skip in &skipped {
        eprintln!(
            "{}: no updater for {} - {}",
            args.zone.display(),
            skip.feed_id,
            skip.reason
        );
    }

    // A deployment missing one of these would come up quietly without its
    // realtime data.
    let unfilled: Vec<&str> = skipped.iter().map(|skip| skip.feed_id.as_str()).collect();
    if !unfilled.is_empty() {
        return Err(format!(
            "no usable credential in {} for: {} - run bin/transit-credentials",
            args.credentials_file.display(),
            unfilled.join(", ")
        )
        .into());
    }

    println!("{}", serde_json::to_string_pretty(&router_config)?);
    Ok(())
}
