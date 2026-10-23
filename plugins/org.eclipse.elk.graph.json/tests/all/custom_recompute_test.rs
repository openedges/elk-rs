use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_json;
use serde_json::{Value, json};
use std::path::Path;

const PLACERS: [&str; 5] = [
    "BRANDES_KOEPF",
    "NETWORK_SIMPLEX",
    "SIMPLE",
    "INTERACTIVE",
    "LINEAR_SEGMENTS",
];

fn enable_recompute(graph: &mut Value, placer: &str) {
    if graph["layoutOptions"].is_null() {
        // layoutOptions takes precedence over legacy properties during import.
        // Preserve hierarchy handling and extension options when adding it.
        graph["layoutOptions"] = graph["properties"]
            .as_object()
            .map(|properties| Value::Object(properties.clone()))
            .unwrap_or_else(|| json!({}));
    }
    graph["layoutOptions"]["elk.algorithm"] = json!("layered");
    graph["layoutOptions"]["elk.layered.nodePlacement.strategy"] = json!("NETWORK_SIMPLEX");
    graph["layoutOptions"]["elk.layered.nodePlacement.networkSimplex.nodeFlexibility.default"] =
        json!("NODE_SIZE");
    graph["layoutOptions"]["elk.layered.nodePlacement.networkSimplex.nodeFlexibility.recomputeNodePlacement"] =
        json!(placer);
    if let Some(children) = graph["children"].as_array_mut() {
        for child in children {
            if child["children"].is_array() {
                enable_recompute(child, placer);
            }
        }
    }
}

fn layout(graph: &Value) -> Value {
    let output = layout_json(&graph.to_string(), "{}").expect("custom recompute layout");
    serde_json::from_str(&output).expect("valid layout JSON")
}

fn number(value: &Value, key: &str) -> f64 {
    let number = value[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key}: {value}"));
    assert!(number.is_finite(), "{key}: {value}");
    number
}

fn ignored_edge_graph() -> Value {
    json!({
        "id": "root",
        "layoutOptions": {"elk.algorithm": "layered", "elk.direction": "RIGHT"},
        "children": [
            {"id": "a", "width": 30, "height": 30,
             "layoutOptions": {"elk.portConstraints": "FIXED_SIDE"},
             "ports": [{"id": "a_out", "width": 4, "height": 4,
                        "layoutOptions": {"elk.port.side": "EAST"}}]},
            {"id": "b", "width": 30, "height": 30,
             "layoutOptions": {"elk.portConstraints": "FIXED_SIDE"},
             "ports": [{"id": "b_in", "width": 4, "height": 4,
                        "layoutOptions": {"elk.port.side": "WEST"}}]}
        ],
        "edges": [{"id": "ignored", "sources": ["a_out"], "targets": ["b_in"],
                   "layoutOptions": {"ignoreEdgeInLayer": true}}]
    })
}

fn assert_original_port_connections(graph: &Value) {
    let edge = &graph["edges"][0];
    assert_eq!(edge["sources"], json!(["a_out"]));
    assert_eq!(edge["targets"], json!(["b_in"]));
    let sections = edge["sections"].as_array().expect("edge sections");
    assert_eq!(sections.len(), 1);
    for (node, point, east) in [
        (&graph["children"][0], &sections[0]["startPoint"], true),
        (&graph["children"][1], &sections[0]["endPoint"], false),
    ] {
        let port = &node["ports"][0];
        // Edge anchors lie on the outward border of a port, centered vertically.
        for (axis, offset) in [
            ("x", if east { number(port, "width") } else { 0.0 }),
            ("y", number(port, "height") / 2.0),
        ] {
            let anchor = number(node, axis) + number(port, axis) + offset;
            assert!((number(point, axis) - anchor).abs() < 1e-6, "{edge}");
        }
    }
}

#[test]
fn recompute_preserves_ignored_layer_and_original_port_connections() {
    for placer in PLACERS {
        let mut graph = ignored_edge_graph();
        enable_recompute(&mut graph, placer);
        let output = layout(&graph);
        // Opposite port margins can shift node boxes within one layer.
        let width = number(&output["children"][0], "width");
        assert!(
            (number(&output["children"][0], "x") - number(&output["children"][1], "x")).abs()
                < width,
            "{placer}: ignored edge must keep its nodes in the same layer"
        );
        assert_original_port_connections(&output);
        graph["edges"][0]["layoutOptions"]["ignoreEdgeInLayer"] = json!(false);
        let normal = layout(&graph);
        assert_original_port_connections(&normal);
        assert!(
            number(&normal["children"][1], "x")
                > number(&normal["children"][0], "x") + number(&normal["children"][0], "width"),
            "{placer}: a normal edge must separate layers"
        );
    }
}

#[test]
fn equal_split_unzipping_and_recompute_preserve_ignored_edge_connections() {
    for placer in PLACERS {
        let mut graph = ignored_edge_graph();
        enable_recompute(&mut graph, placer);
        graph["layoutOptions"]["elk.layered.layerUnzipping.strategy"] = json!("ALTERNATING");
        graph["layoutOptions"]["elk.layered.layerUnzipping.layerSplit"] = json!(2);
        let output = layout(&graph);
        assert!(
            (number(&output["children"][0], "x") - number(&output["children"][1], "x")).abs()
                > number(&output["children"][0], "width"),
            "{placer}: two nodes must unzip at the split=2 boundary"
        );
        assert_original_port_connections(&output);
    }
}

fn assert_graph_preserved(input: &Value, output: &Value) {
    assert_eq!(input["id"], output["id"]);
    for key in ["x", "y", "width", "height"] {
        number(output, key);
    }
    for key in ["children", "ports", "edges"] {
        let empty = Vec::new();
        let before = input[key].as_array().unwrap_or(&empty);
        let after = output[key].as_array().unwrap_or(&empty);
        assert_eq!(before.len(), after.len(), "{key} in {}", input["id"]);
        for item in before {
            let actual = after
                .iter()
                .find(|a| a["id"] == item["id"])
                .expect("retained id");
            if key == "edges" {
                assert_eq!(item["sources"], actual["sources"]);
                assert_eq!(item["targets"], actual["targets"]);
                let sections = actual["sections"].as_array().expect("routed edge");
                assert!(!sections.is_empty(), "{}", item["id"]);
                for section in sections {
                    for point in ["startPoint", "endPoint"] {
                        number(&section[point], "x");
                        number(&section[point], "y");
                    }
                }
            } else {
                assert_graph_preserved(item, actual);
            }
        }
    }
}

#[test]
fn all_extension_fixtures_support_each_recompute_strategy() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures: Vec<_> = std::fs::read_dir(dir)
        .expect("extension fixtures")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".elk.json"))
        .collect();
    fixtures.sort();
    assert_eq!(fixtures.len(), 18);
    for path in fixtures {
        for placer in PLACERS {
            let mut graph: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            enable_recompute(&mut graph, placer);
            let output = layout_json(&graph.to_string(), "{}")
                .unwrap_or_else(|error| panic!("{} / {placer}: {error}", path.display()));
            let output: Value = serde_json::from_str(&output).expect("valid layout JSON");
            assert_graph_preserved(&graph, &output);
        }
    }
}
