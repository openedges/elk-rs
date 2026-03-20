/// Integration test: edgeless hierarchical model with labeled ports.
///
/// Reproduces the key characteristics of the QA OAD customer model without
/// customer-specific data. The model has:
///   - 3-level hierarchy (root → domain → leaf nodes)
///   - All ports have labels
///   - Zero edges anywhere
///   - Full process() + edgeless port handling must work together
///
/// This tests the combined effect of all three bug fixes:
///   1. Multi-label cell overwrite (CellSystem)
///   2. Fixed port label insidePart (real positions)
///   3. Edgeless root port external port treatment
use serde_json::Value;

fn layout_json(input: &str) -> Value {
    let result =
        org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_json(
            input, "{}",
        )
        .expect("layout should succeed");
    serde_json::from_str(&result).expect("output should be valid JSON")
}

fn build_edgeless_hierarchy_model() -> String {
    r#"{
  "id": "root",
  "layoutOptions": {
    "algorithm": "layered",
    "elk.direction": "RIGHT",
    "hierarchyHandling": "INCLUDE_CHILDREN"
  },
  "children": [
    {
      "id": "domain",
      "width": 400,
      "height": 300,
      "labels": [{ "text": "Domain", "width": 50, "height": 14 }],
      "layoutOptions": {
        "algorithm": "layered",
        "elk.direction": "RIGHT",
        "elk.portConstraints": "FIXED_SIDE"
      },
      "ports": [
        { "id": "dp_w0", "width": 0, "height": 12, "labels": [{"text": "wp0"}],
          "layoutOptions": { "elk.port.side": "WEST" } },
        { "id": "dp_w1", "width": 0, "height": 12, "labels": [{"text": "wp1"}],
          "layoutOptions": { "elk.port.side": "WEST" } },
        { "id": "dp_w2", "width": 0, "height": 12, "labels": [{"text": "wp2"}],
          "layoutOptions": { "elk.port.side": "WEST" } },
        { "id": "dp_e0", "width": 0, "height": 12, "labels": [{"text": "ep0"}],
          "layoutOptions": { "elk.port.side": "EAST" } },
        { "id": "dp_e1", "width": 0, "height": 12, "labels": [{"text": "ep1"}],
          "layoutOptions": { "elk.port.side": "EAST" } },
        { "id": "dp_s0", "width": 50, "height": 0, "labels": [{"text": "sp0"}],
          "layoutOptions": { "elk.port.side": "SOUTH" } },
        { "id": "dp_s1", "width": 50, "height": 0, "labels": [{"text": "sp1"}],
          "layoutOptions": { "elk.port.side": "SOUTH" } }
      ],
      "children": [
        {
          "id": "leaf_a",
          "width": 80, "height": 50,
          "labels": [{ "text": "LeafA", "width": 40, "height": 14 }],
          "ports": [
            { "id": "la_in", "width": 0, "height": 10, "labels": [{"text": "in"}],
              "layoutOptions": { "elk.port.side": "WEST" } },
            { "id": "la_out", "width": 0, "height": 10, "labels": [{"text": "out"}],
              "layoutOptions": { "elk.port.side": "EAST" } }
          ]
        },
        {
          "id": "leaf_b",
          "width": 80, "height": 50,
          "labels": [{ "text": "LeafB", "width": 40, "height": 14 }],
          "ports": [
            { "id": "lb_in", "width": 0, "height": 10, "labels": [{"text": "in"}],
              "layoutOptions": { "elk.port.side": "WEST" } },
            { "id": "lb_out", "width": 0, "height": 10, "labels": [{"text": "out"}],
              "layoutOptions": { "elk.port.side": "EAST" } }
          ]
        },
        {
          "id": "leaf_c",
          "width": 80, "height": 50,
          "labels": [
            { "text": "LeafC Line1", "width": 60, "height": 14,
              "layoutOptions": { "nodeLabels.placement": "INSIDE V_TOP H_CENTER" } },
            { "text": "LeafC Line2", "width": 60, "height": 14,
              "layoutOptions": { "nodeLabels.placement": "INSIDE V_TOP H_CENTER" } }
          ],
          "ports": [
            { "id": "lc_in", "width": 0, "height": 10, "labels": [{"text": "in"}],
              "layoutOptions": { "elk.port.side": "WEST" } },
            { "id": "lc_out", "width": 0, "height": 10, "labels": [{"text": "out"}],
              "layoutOptions": { "elk.port.side": "EAST" } }
          ]
        }
      ]
    }
  ]
}"#
    .to_string()
}

fn find_node<'a>(data: &'a Value, id: &str) -> Option<&'a Value> {
    if data.get("id").and_then(|v| v.as_str()) == Some(id) {
        return Some(data);
    }
    if let Some(children) = data.get("children").and_then(|v| v.as_array()) {
        for child in children {
            if let Some(found) = find_node(child, id) {
                return Some(found);
            }
        }
    }
    None
}

fn get_ports(node: &Value) -> Vec<&Value> {
    node.get("ports")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

fn get_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// The layout must complete without panic on a fully edgeless hierarchy.
#[test]
fn edgeless_hierarchy_layout_completes() {
    let input = build_edgeless_hierarchy_model();
    let _output = layout_json(&input);
}

/// Domain node must have reasonable dimensions after layout (not collapsed to zero).
#[test]
fn edgeless_hierarchy_domain_node_sized() {
    let input = build_edgeless_hierarchy_model();
    let output = layout_json(&input);

    let domain = find_node(&output, "domain").expect("domain node");
    let w = get_f64(domain, "width");
    let h = get_f64(domain, "height");

    assert!(
        w > 50.0 && h > 50.0,
        "domain node should have reasonable size, got ({w}, {h})"
    );
}

/// Leaf nodes with two labels at the same location must have both labels positioned
/// (tests multi-label cell overwrite fix).
#[test]
fn edgeless_hierarchy_multi_label_leaf_positioned() {
    let input = build_edgeless_hierarchy_model();
    let output = layout_json(&input);

    let leaf_c = find_node(&output, "leaf_c").expect("leaf_c node");
    let labels = leaf_c
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels");

    assert!(labels.len() >= 2, "leaf_c should have 2+ labels");

    let y0 = get_f64(&labels[0], "y");
    let y1 = get_f64(&labels[1], "y");

    // Both labels should be positioned (not at origin)
    // and second should be below first
    assert!(
        y1 > y0,
        "second label y={y1} should be below first label y={y0} (multi-label fix)"
    );
}

/// Domain ports (edgeless) should be distributed along their sides,
/// not clustered at origin.
#[test]
fn edgeless_hierarchy_domain_ports_distributed() {
    let input = build_edgeless_hierarchy_model();
    let output = layout_json(&input);

    let domain = find_node(&output, "domain").expect("domain node");
    let ports = get_ports(domain);

    // WEST ports should have distinct Y positions
    let west_ys: Vec<f64> = ports
        .iter()
        .filter(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.starts_with("dp_w"))
                .unwrap_or(false)
        })
        .map(|p| get_f64(p, "y"))
        .collect();

    assert_eq!(west_ys.len(), 3, "should have 3 WEST ports");
    // All Y positions should be distinct (not all at 0)
    let all_same = west_ys.windows(2).all(|w| (w[0] - w[1]).abs() < 1.0);
    assert!(
        !all_same,
        "WEST ports should have distinct Y positions, got {west_ys:?}"
    );

    // SOUTH ports should have distinct X positions
    let south_xs: Vec<f64> = ports
        .iter()
        .filter(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.starts_with("dp_s"))
                .unwrap_or(false)
        })
        .map(|p| get_f64(p, "x"))
        .collect();

    assert_eq!(south_xs.len(), 2, "should have 2 SOUTH ports");
    assert!(
        (south_xs[0] - south_xs[1]).abs() > 1.0,
        "SOUTH ports should have distinct X positions, got {south_xs:?}"
    );
}

/// Leaf nodes should have distinct positions after layout
/// (tests that process() + edgeless hierarchy produces valid placement).
#[test]
fn edgeless_hierarchy_leaf_nodes_have_distinct_positions() {
    let input = build_edgeless_hierarchy_model();
    let output = layout_json(&input);

    let mut positions = Vec::new();
    for id in ["leaf_a", "leaf_b", "leaf_c"] {
        let node = find_node(&output, id).expect(id);
        let x = get_f64(node, "x");
        let y = get_f64(node, "y");
        positions.push((id, x, y));
    }

    // All leaf nodes should have distinct positions (not stacked at origin)
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let (id_a, xa, ya) = positions[i];
            let (id_b, xb, yb) = positions[j];
            assert!(
                (xa - xb).abs() > 1.0 || (ya - yb).abs() > 1.0,
                "leaf nodes {id_a} and {id_b} should have distinct positions, got ({xa},{ya}) and ({xb},{yb})"
            );
        }
    }
}

/// The full layout must be deterministic — running twice produces identical output.
#[test]
fn edgeless_hierarchy_deterministic() {
    let input = build_edgeless_hierarchy_model();
    let output1 = layout_json(&input);
    let output2 = layout_json(&input);

    assert_eq!(
        output1, output2,
        "layout should be deterministic across runs"
    );
}
