//! The generated style has to reproduce the hand-written one it replaces.

use serde_json::{json, Value as Json};

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
fn reproduces_the_hand_written_style() {
    let mut expected: Json =
        serde_json::from_str(include_str!("../../assets/styles/basic-v3.json"))
            .expect("basic-v3.json parses");
    let mut generated = mlss::headway::style().to_json();

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
    // `id` and `metadata` name the style rather than describing how it draws.
    for key in ["id", "metadata"] {
        generated.as_object_mut().unwrap().remove(key);
        expected.as_object_mut().unwrap().remove(key);
    }

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
        "{} differences from basic-v3.json:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}
