//! The generated style has to reproduce the checked-in Headway reference.

use serde_json::{json, Value as Json};

const REFERENCE: &str = include_str!("fixtures/headway.json");

/// Smooths over two things the hand-written style is inconsistent about and
/// maplibre doesn't care about: integers written as floats, and the order of
/// the clauses inside an `all` filter.
fn normalize(value: &Json) -> Json {
    match value {
        Json::Number(number) => json!(number.as_f64().expect("finite")),
        Json::Array(items) => {
            let mut items: Vec<Json> = items.iter().map(normalize).collect();
            let single_valued_membership = items.len() == 3
                && matches!(items[0].as_str(), Some("in") | Some("!in"))
                && items[1].is_string();
            if single_valued_membership {
                items[0] = json!(if items[0] == json!("in") { "==" } else { "!=" });
            }
            if items.first() == Some(&json!("all")) {
                items[1..].sort_by_key(ToString::to_string);
            }
            let route_network_match = items.len() >= 4
                && items.first() == Some(&json!("match"))
                && items.get(1) == Some(&json!(["get", "route_1_network"]));
            if route_network_match {
                let fallback = items.pop().expect("match has a fallback");
                let mut cases = Vec::new();
                for pair in items[2..].chunks_exact(2) {
                    let networks = match &pair[0] {
                        Json::String(network) => vec![network.clone()],
                        Json::Array(networks) => networks
                            .iter()
                            .map(|network| network.as_str().expect("network name").to_owned())
                            .collect(),
                        _ => panic!("route-network match label"),
                    };
                    cases.extend(
                        networks
                            .into_iter()
                            .map(|network| (network, pair[1].clone())),
                    );
                }
                cases.sort_by(|left, right| left.0.cmp(&right.0));
                items.truncate(2);
                for (network, value) in cases {
                    items.push(json!(network));
                    items.push(value);
                }
                items.push(fallback);
            }
            Json::Array(items)
        }
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), normalize(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn differences(path: &str, generated: &Json, expected: &Json, found: &mut Vec<String>) {
    match (generated, expected) {
        (Json::Object(generated), Json::Object(expected)) => {
            let mut keys: Vec<&String> = generated.keys().chain(expected.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let missing = Json::Null;
                differences(
                    &format!("{path}.{key}"),
                    generated.get(key).unwrap_or(&missing),
                    expected.get(key).unwrap_or(&missing),
                    found,
                );
            }
        }
        (Json::Array(generated), Json::Array(expected)) if generated.len() == expected.len() => {
            for (index, (generated, expected)) in generated.iter().zip(expected).enumerate() {
                differences(&format!("{path}[{index}]"), generated, expected, found);
            }
        }
        (generated, expected) if generated != expected => {
            found.push(format!(
                "{path}\n     generated: {generated}\n      expected: {expected}"
            ));
        }
        _ => {}
    }
}

#[test]
fn matches_the_checked_in_headway_style_byte_for_byte() {
    assert_eq!(mlss::headway::style().to_json_string(), REFERENCE);
}

#[test]
fn matches_the_checked_in_headway_style_semantically() {
    let expected: Json = serde_json::from_str(REFERENCE).expect("headway.json parses");
    let generated = mlss::headway::style().to_json();

    let expected_ids: Vec<String> = expected["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|layer| layer["id"].as_str().unwrap().to_owned())
        .collect();
    let generated_ids: Vec<String> = generated["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|layer| layer["id"].as_str().unwrap().to_owned())
        .collect();
    let mut found = Vec::new();
    if generated_ids != expected_ids {
        found.push(format!(
            "style.layers order\n     generated: {generated_ids:?}\n      expected: {expected_ids:?}"
        ));
    }
    differences(
        "style",
        &normalize(&generated),
        &normalize(&expected),
        &mut found,
    );
    assert!(
        found.is_empty(),
        "{} differences from headway.json:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}
