//! Renders a zone's OTP `router-config.json` on stdout.
//!
//! The updaters aren't a field of the zone document - they're derived from the
//! realtime feeds hanging off its static ones, so there's no second copy of the
//! same facts to drift out of step with the feed list. This is what turns the
//! one into the other, and the deployment scripts run it where they used to
//! read a stored section. See [`transit_zone::router_config`].
//!
//! It lives in this crate rather than in `gtfout` so the deploy path builds
//! only serde and csv: rendering a config needs the zone, not the atlas.

use transit_zone::zone::Zone;
use transit_zone::Result;

use std::path::PathBuf;

fn main() -> Result<()> {
    let Some(zone_path) = zone_arg()? else {
        eprintln!("usage: zone-router-config --zone <zone.json>");
        std::process::exit(2);
    };

    let zone = Zone::load(&zone_path)?;
    let (router_config, skipped) = zone.router_config();

    // Not an error: a realtime feed OTP has nowhere to put the credential for
    // is still a zone worth deploying. But it drops off silently otherwise, and
    // "why is there no realtime" is a miserable thing to debug from a
    // ConfigMap, so name the feed and the reason.
    for skip in &skipped {
        eprintln!(
            "{}: no updater for {} - {}",
            zone_path.display(),
            skip.feed_id,
            skip.reason
        );
    }

    println!("{}", serde_json::to_string_pretty(&router_config)?);
    Ok(())
}

/// The `--zone <path>` argument.
///
/// Named rather than positional because the deployment scripts pass a path that
/// came out of a `find`, and a bare argument that happened to start with a dash
/// would be read as a flag.
fn zone_arg() -> Result<Option<PathBuf>> {
    let mut args = std::env::args().skip(1);
    let mut zone_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--zone" => {
                zone_path = Some(PathBuf::from(args.next().ok_or("--zone needs a path")?));
            }
            other => return Err(format!("unexpected argument {other:?}").into()),
        }
    }
    Ok(zone_path)
}
