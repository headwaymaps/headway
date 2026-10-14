//! Prints the Headway style as JSON.

fn main() {
    print!("{}", mlss::headway::style().to_json_string());
}
