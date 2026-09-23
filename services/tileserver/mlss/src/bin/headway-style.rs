//! Prints the Headway style as JSON.

fn main() {
    println!("{}", mlss::headway::style().to_json_string());
}
