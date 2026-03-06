use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_json;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn fixtures_dir() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .join("tests/fixtures")
        .display()
        .to_string()
}

fn collect_fixture_pairs(filter: fn(&str) -> bool) -> Vec<(String, String, String)> {
    let dir = fixtures_dir();
    let mut pairs = Vec::new();

    let entries: Vec<_> = fs::read_dir(&dir)
        .expect("read fixtures dir")
        .filter_map(|e| e.ok())
        .collect();

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".elk.json") && !name.contains(".layout.") && filter(&name) {
            let input_path = entry.path().display().to_string();
            let expected_name = name.replace(".elk.json", ".elk.layout.json");
            let expected_path = Path::new(&dir).join(&expected_name).display().to_string();
            if Path::new(&expected_path).exists() {
                pairs.push((name, input_path, expected_path));
            }
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn normalize_json(value: &mut Value) {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                let rounded = (f * 10000.0).round() / 10000.0;
                *value = serde_json::json!(rounded);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                normalize_json(item);
            }
        }
        Value::Object(map) => {
            let internal_keys: Vec<String> = map
                .keys()
                .filter(|k| k.starts_with('$'))
                .cloned()
                .collect();
            for k in internal_keys {
                map.remove(&k);
            }
            for (_k, v) in map.iter_mut() {
                normalize_json(v);
            }
        }
        _ => {}
    }
}

fn run_fixtures(pairs: &[(String, String, String)]) {
    assert!(!pairs.is_empty(), "no fixture pairs found");

    let mut passed = 0;
    let mut failed = Vec::new();

    for (name, input_path, expected_path) in pairs {
        let input_json = fs::read_to_string(input_path)
            .unwrap_or_else(|e| panic!("read {input_path}: {e}"));
        let expected_json = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("read {expected_path}: {e}"));

        let result = layout_json(&input_json, "{}");

        match result {
            Ok(output_json) => {
                let mut actual: Value = serde_json::from_str(&output_json)
                    .unwrap_or_else(|e| panic!("parse output for {name}: {e}"));
                let mut expected: Value = serde_json::from_str(&expected_json)
                    .unwrap_or_else(|e| panic!("parse expected for {name}: {e}"));

                normalize_json(&mut actual);
                normalize_json(&mut expected);

                if actual == expected {
                    passed += 1;
                } else {
                    let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
                    let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();
                    let actual_lines: Vec<&str> = actual_pretty.lines().collect();
                    let expected_lines: Vec<&str> = expected_pretty.lines().collect();
                    let mut diff_count = 0;
                    for (i, (a, e)) in
                        actual_lines.iter().zip(expected_lines.iter()).enumerate()
                    {
                        if a != e && diff_count < 5 {
                            println!("  {name} line {}: actual={a} expected={e}", i + 1);
                            diff_count += 1;
                        }
                    }
                    if actual_lines.len() != expected_lines.len() {
                        println!(
                            "  {name}: line count actual={} expected={}",
                            actual_lines.len(),
                            expected_lines.len()
                        );
                    }
                    failed.push(format!("{name}: output differs from expected"));
                }
            }
            Err(err) => {
                failed.push(format!("{name}: layout error: {err}"));
            }
        }
    }

    println!("\nFixture results: {passed}/{} passed", pairs.len());
    for f in &failed {
        println!("  FAIL: {f}");
    }

    assert!(
        failed.is_empty(),
        "{} of {} fixtures failed",
        failed.len(),
        pairs.len()
    );
}

fn is_ignore_edge_in_layer(name: &str) -> bool {
    name.contains("ignoreEdgeInLayer")
}

#[test]
fn ignore_edge_in_layer_fixtures_produce_expected_layout() {
    let pairs = collect_fixture_pairs(is_ignore_edge_in_layer);
    run_fixtures(&pairs);
}

#[test]
fn in_layer_edge_routing_fixtures_produce_expected_layout() {
    let pairs = collect_fixture_pairs(|name| !is_ignore_edge_in_layer(name));
    run_fixtures(&pairs);
}
