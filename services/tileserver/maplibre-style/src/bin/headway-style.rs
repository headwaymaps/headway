//! Prints a revision of the Headway style as JSON: `headway-style v3|v4`.

use maplibre_style::headway::Version;

fn main() {
    let version = match std::env::args().nth(1).as_deref() {
        Some("v3") => Version::V3,
        Some("v4") => Version::V4,
        other => {
            eprintln!("usage: headway-style v3|v4 (got {other:?})");
            std::process::exit(2);
        }
    };
    print!(
        "{}",
        maplibre_style::headway::style(version).to_json_string()
    );
}
