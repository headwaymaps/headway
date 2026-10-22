//! Prints the Headway style as JSON.

fn main() {
    print!("{}", maplibre_style::headway::style().to_json_string());
}
