//! The facade must expose the JSON API and the re-exported crates end to end.

use serde_json::Value;

const GRAPH: &str = r#"{
  "id": "root",
  "layoutOptions": { "elk.algorithm": "layered" },
  "children": [
    { "id": "n1", "width": 30, "height": 30 },
    { "id": "n2", "width": 30, "height": 30 },
    { "id": "n3", "width": 30, "height": 30 }
  ],
  "edges": [
    { "id": "e1", "sources": ["n1"], "targets": ["n2"] },
    { "id": "e2", "sources": ["n1"], "targets": ["n3"] }
  ]
}"#;

#[test]
fn layout_json_positions_every_child() {
    let output = elk_layout::layout_json(GRAPH, "{}").expect("layout succeeds");
    let graph: Value = serde_json::from_str(&output).expect("output is JSON");
    let children = graph["children"].as_array().expect("children array");
    assert_eq!(children.len(), 3);
    for child in children {
        assert!(child["x"].is_number(), "{child} has no x");
        assert!(child["y"].is_number(), "{child} has no y");
    }
    assert!(graph["width"].as_f64().unwrap_or(0.0) > 0.0);
    assert!(graph["height"].as_f64().unwrap_or(0.0) > 0.0);
}

#[test]
fn global_options_are_merged_as_defaults() {
    let graph = GRAPH.replace(r#""layoutOptions": { "elk.algorithm": "layered" },"#, "");
    let output = elk_layout::layout_json(&graph, r#"{ "elk.algorithm": "force" }"#)
        .expect("layout with global options succeeds");
    let graph: Value = serde_json::from_str(&output).expect("output is JSON");
    assert_eq!(graph["children"].as_array().map(Vec::len), Some(3));
}

#[test]
fn metadata_queries_list_the_ported_algorithms() {
    let algorithms = elk_layout::known_layout_algorithms().expect("algorithms");
    for id in [
        "org.eclipse.elk.layered",
        "org.eclipse.elk.force",
        "org.eclipse.elk.mrtree",
        "org.eclipse.elk.radial",
        "org.eclipse.elk.rectpacking",
    ] {
        assert!(algorithms.contains(id), "{id} missing from {algorithms}");
    }
    assert!(elk_layout::known_layout_options().expect("options").contains("org.eclipse.elk.algorithm"));
    assert!(elk_layout::known_layout_categories().expect("categories").contains("org.eclipse.elk.layered"));
}

#[test]
fn re_exported_crates_are_reachable() {
    // Compile-time check that the module paths exist; the metadata service lives in `core`.
    elk_layout::ensure_initialized();
    let _ = std::any::type_name::<elk_layout::graph_json::org::eclipse::elk::graph::json::JsonImporter>();
    let _ = std::any::type_name::<elk_layout::core::org::eclipse::elk::core::data::LayoutMetaDataService>();
}
