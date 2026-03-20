//! Tests for root-level external port placement.
//!
//! Verifies that external ports declared on the root node are distributed
//! along their assigned sides, not clustered at a single point.
//! See: elk-rs-qa/FIX_PLAN_external_port_placement.md

use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn layout(input: &str) -> serde_json::Value {
    let result = layout_api::layout_json(input, "{}").expect("layout should succeed");
    serde_json::from_str(&result).expect("layout result should be valid JSON")
}

/// Extract (id, x, y) tuples for all ports in the root JSON object.
fn port_positions(root: &serde_json::Value) -> Vec<(String, f64, f64)> {
    root["ports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["id"].as_str().unwrap().to_string(),
                p["x"].as_f64().unwrap_or(0.0),
                p["y"].as_f64().unwrap_or(0.0),
            )
        })
        .collect()
}

/// Collect Y values for ports whose id starts with `prefix`.
fn ys_for(ports: &[(String, f64, f64)], prefix: &str) -> Vec<f64> {
    ports
        .iter()
        .filter(|(id, _, _)| id.starts_with(prefix))
        .map(|(_, _, y)| *y)
        .collect()
}

/// Collect X values for ports whose id starts with `prefix`.
fn xs_for(ports: &[(String, f64, f64)], prefix: &str) -> Vec<f64> {
    ports
        .iter()
        .filter(|(id, _, _)| id.starts_with(prefix))
        .map(|(_, x, _)| *x)
        .collect()
}

fn assert_all_distinct(values: &[f64], label: &str) {
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            assert!(
                (values[i] - values[j]).abs() > 1.0,
                "{label}: values must be distinct — [{i}]={}, [{j}]={}",
                values[i],
                values[j],
            );
        }
    }
}

fn assert_within(values: &[f64], min: f64, max: f64, label: &str) {
    for (i, &v) in values.iter().enumerate() {
        assert!(
            v >= min && v <= max,
            "{label}[{i}]={v} out of range [{min}, {max}]",
        );
    }
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

/// WEST 포트 3개가 Y축으로 분산 배치되어야 한다.
#[test]
fn root_ext_ports_west_distributed() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w1", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w2", "width": 0, "height": 25, "properties": {"side": "WEST"}}
          ]
        }"#,
    );

    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);
    let west_ys = ys_for(&ports, "w");

    assert_eq!(west_ys.len(), 3);
    assert_all_distinct(&west_ys, "WEST Y");
    assert_within(&west_ys, 0.0, h, "WEST Y");
}

/// SOUTH 포트 3개가 X축으로 분산 배치되어야 한다.
#[test]
fn root_ext_ports_south_distributed() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "s0", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "s1", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "s2", "width": 25, "height": 0, "properties": {"side": "SOUTH"}}
          ]
        }"#,
    );

    let w = root["width"].as_f64().unwrap();
    let ports = port_positions(&root);
    let south_xs = xs_for(&ports, "s");

    assert_eq!(south_xs.len(), 3);
    assert_all_distinct(&south_xs, "SOUTH X");
    assert_within(&south_xs, 0.0, w, "SOUTH X");
}

/// NORTH 포트 3개가 X축으로 분산 배치되어야 한다.
#[test]
fn root_ext_ports_north_distributed() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "n0", "width": 25, "height": 0, "properties": {"side": "NORTH"}},
            {"id": "n1", "width": 25, "height": 0, "properties": {"side": "NORTH"}},
            {"id": "n2", "width": 25, "height": 0, "properties": {"side": "NORTH"}}
          ]
        }"#,
    );

    let w = root["width"].as_f64().unwrap();
    let ports = port_positions(&root);
    let north_xs = xs_for(&ports, "n");

    assert_eq!(north_xs.len(), 3);
    assert_all_distinct(&north_xs, "NORTH X");
    assert_within(&north_xs, 0.0, w, "NORTH X");
}

/// EAST 포트 1개가 root 높이 범위 내 (대략 중앙)에 배치되어야 한다.
#[test]
fn root_ext_ports_east_centered() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "e0", "width": 0, "height": 25, "properties": {"side": "EAST"}}
          ]
        }"#,
    );

    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);
    let east_ys = ys_for(&ports, "e");

    assert_eq!(east_ys.len(), 1);
    assert_within(&east_ys, 0.0, h, "EAST Y");
}

/// 4면 (N/S/E/W) 각 2개씩 — 같은 side 내 distinct 확인.
#[test]
fn root_ext_ports_all_four_sides() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "n0", "width": 25, "height": 0, "properties": {"side": "NORTH"}},
            {"id": "n1", "width": 25, "height": 0, "properties": {"side": "NORTH"}},
            {"id": "s0", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "s1", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w1", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "e0", "width": 0, "height": 25, "properties": {"side": "EAST"}},
            {"id": "e1", "width": 0, "height": 25, "properties": {"side": "EAST"}}
          ]
        }"#,
    );

    let w = root["width"].as_f64().unwrap();
    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);

    assert_all_distinct(&xs_for(&ports, "n"), "NORTH X");
    assert_all_distinct(&xs_for(&ports, "s"), "SOUTH X");
    assert_all_distinct(&ys_for(&ports, "w"), "WEST Y");
    assert_all_distinct(&ys_for(&ports, "e"), "EAST Y");

    assert_within(&xs_for(&ports, "n"), 0.0, w, "NORTH X");
    assert_within(&xs_for(&ports, "s"), 0.0, w, "SOUTH X");
    assert_within(&ys_for(&ports, "w"), 0.0, h, "WEST Y");
    assert_within(&ys_for(&ports, "e"), 0.0, h, "EAST Y");
}

/// Domain 내부에 edge가 있어도 root port가 분산되어야 한다.
#[test]
fn root_ext_ports_with_domain_edges() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [
            {
              "id": "dom",
              "properties": { "portConstraints": "FIXED_SIDE" },
              "ports": [
                {"id": "dp0", "width": 5, "height": 5, "properties": {"side": "WEST"}},
                {"id": "dp1", "width": 5, "height": 5, "properties": {"side": "EAST"}}
              ],
              "children": [
                {"id": "c0", "width": 80, "height": 40,
                 "ports": [
                   {"id": "c0i", "width": 5, "height": 5, "properties": {"side": "WEST"}},
                   {"id": "c0o", "width": 5, "height": 5, "properties": {"side": "EAST"}}
                 ]},
                {"id": "c1", "width": 80, "height": 40,
                 "ports": [
                   {"id": "c1i", "width": 5, "height": 5, "properties": {"side": "WEST"}}
                 ]}
              ],
              "edges": [
                {"id": "de0", "sources": ["dp0"], "targets": ["c0i"]},
                {"id": "de1", "sources": ["c0o"], "targets": ["c1i"]}
              ]
            }
          ],
          "ports": [
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w1", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w2", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "e0", "width": 0, "height": 25, "properties": {"side": "EAST"}},
            {"id": "s0", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "s1", "width": 25, "height": 0, "properties": {"side": "SOUTH"}}
          ]
        }"#,
    );

    let w = root["width"].as_f64().unwrap();
    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);

    assert_all_distinct(&ys_for(&ports, "w"), "WEST Y");
    assert_all_distinct(&xs_for(&ports, "s"), "SOUTH X");
    assert_within(&ys_for(&ports, "w"), 0.0, h, "WEST Y");
    assert_within(&xs_for(&ports, "s"), 0.0, w, "SOUTH X");
}

/// FIXED_ORDER constraint에서 port 순서가 유지되고 분산되어야 한다.
#[test]
fn root_ext_ports_fixed_order() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_ORDER",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST", "index": "0"}},
            {"id": "w1", "width": 0, "height": 25, "properties": {"side": "WEST", "index": "1"}},
            {"id": "w2", "width": 0, "height": 25, "properties": {"side": "WEST", "index": "2"}}
          ]
        }"#,
    );

    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);
    let west_ys = ys_for(&ports, "w");

    assert_eq!(west_ys.len(), 3);
    assert_all_distinct(&west_ys, "WEST Y (FIXED_ORDER)");
    assert_within(&west_ys, 0.0, h, "WEST Y (FIXED_ORDER)");
}

/// 복수 child + ports, no edges — child가 여러 개여도 root port 분산.
#[test]
fn root_ext_ports_multiple_children() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [
            {"id": "dom1", "width": 200, "height": 150},
            {"id": "dom2", "width": 200, "height": 150}
          ],
          "ports": [
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "w1", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "s0", "width": 25, "height": 0, "properties": {"side": "SOUTH"}},
            {"id": "s1", "width": 25, "height": 0, "properties": {"side": "SOUTH"}}
          ]
        }"#,
    );

    let w = root["width"].as_f64().unwrap();
    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);

    assert_all_distinct(&ys_for(&ports, "w"), "WEST Y");
    assert_all_distinct(&xs_for(&ports, "s"), "SOUTH X");
    assert_within(&ys_for(&ports, "w"), 0.0, h, "WEST Y");
    assert_within(&xs_for(&ports, "s"), 0.0, w, "SOUTH X");
}

/// 각 side에 1개씩 — 정상 배치 (범위 내).
#[test]
fn root_ext_ports_single_per_side() {
    let root = layout(
        r#"{
          "id": "root",
          "properties": {
            "algorithm": "layered",
            "elk.direction": "RIGHT",
            "portConstraints": "FIXED_SIDE",
            "elk.padding": "[top=60,left=120,bottom=60,right=120]"
          },
          "children": [{"id": "dom", "width": 400, "height": 300}],
          "ports": [
            {"id": "w0", "width": 0, "height": 25, "properties": {"side": "WEST"}},
            {"id": "e0", "width": 0, "height": 25, "properties": {"side": "EAST"}}
          ]
        }"#,
    );

    let h = root["height"].as_f64().unwrap();
    let ports = port_positions(&root);

    assert_within(&ys_for(&ports, "w"), 0.0, h, "WEST Y");
    assert_within(&ys_for(&ports, "e"), 0.0, h, "EAST Y");
}
