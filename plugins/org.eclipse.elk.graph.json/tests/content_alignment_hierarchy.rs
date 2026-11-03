//! Reference cases for content alignment with a minimum size under
//! `INCLUDE_CHILDREN`, from elk-rs issue #3. The expected layouts were produced
//! by elkjs 0.12.0 and confirmed against the compiled Java ELK 0.12.0 reference;
//! see tests/content_alignment_hierarchy.md for provenance and gate results.

use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
use serde_json::Value;
use std::path::PathBuf;

fn fixture_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/content_alignment_hierarchy")
}

fn assert_layout_equal(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-6,
                "{path}: expected {a}, actual {b}"
            );
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: different object keys");
            for (key, value) in a {
                assert_layout_equal(
                    value,
                    b.get(key)
                        .unwrap_or_else(|| panic!("{path}: missing {key}")),
                    &format!("{path}/{key}"),
                );
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: different array lengths");
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                assert_layout_equal(a, b, &format!("{path}/{index}"));
            }
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}

fn layout_case(name: &str) -> Value {
    let input =
        std::fs::read_to_string(fixture_directory().join(format!("{name}.input.json"))).unwrap();
    serde_json::from_str(&layout_api::layout_json(&input, "{}").unwrap()).unwrap()
}

/// Every child must stay inside its parent. Edge geometry is covered by the
/// reference comparison and, for the wired case, by the explicit coordinates below.
fn assert_children_contained(node: &Value, path: &str) {
    let number = |value: &Value, key: &str| value[key].as_f64().unwrap_or(0.0);
    let width = number(node, "width");
    let height = number(node, "height");
    if let Some(children) = node["children"].as_array() {
        for child in children {
            let id = child["id"].as_str().unwrap_or("?");
            let (x, y) = (number(child, "x"), number(child, "y"));
            let (w, h) = (number(child, "width"), number(child, "height"));
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= width + 1e-6 && y + h <= height + 1e-6,
                "{path}/{id}: child ({x}, {y}) {w}x{h} outside parent {width}x{height}"
            );
            assert_children_contained(child, &format!("{path}/{id}"));
        }
    }
}

/// Compares the layout with the reference file and checks containment, so every
/// reference case also fails if a child leaves its parent.
fn check_case(name: &str) {
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture_directory().join(format!("{name}.expected.json")))
            .unwrap(),
    )
    .unwrap();
    let actual = layout_case(name);
    assert_layout_equal(&expected, &actual, name);
    assert_children_contained(&actual, name);
}

macro_rules! reference_tests {
    ($($name:ident),* $(,)?) => { $(
        #[test]
        fn $name() { check_case(stringify!($name)); }
    )* };
}

reference_tests!(
    down_h_center,
    up_h_center,
    down_v_center,
    down_h_left,
    down_nested_h_center,
    down_ports_h_center,
    down_separate_children,
    right_h_center,
    left_h_center,
    default_direction_h_center,
);

#[test]
fn issue_3_child_is_centered_inside_the_minimum_size_parent() {
    // The minimum size (300, 40) is applied in the layered graph's internal
    // orientation, so under DOWN the parent becomes 124 x 300 and H_CENTER
    // centers the child along the 300 axis: (12, 12 + (300 - 64) / 2).
    let layout = layout_case("down_h_center");
    let parent = &layout["children"][0];
    let child = &parent["children"][0];
    assert_eq!(parent["width"], 124.0);
    assert_eq!(parent["height"], 300.0);
    assert_eq!(child["x"], 12.0);
    assert_eq!(child["y"], 130.0);

    // With a port on the parent wired to a port on the child, the straight
    // edge runs from the parent's port into the child's port.
    let layout = layout_case("down_ports_h_center");
    let parent = &layout["children"][0];
    let section = &parent["edges"][0]["sections"][0];
    assert_eq!(parent["ports"][0]["x"], 57.0);
    assert_eq!(parent["children"][0]["x"], 12.0);
    assert_eq!(parent["children"][0]["ports"][0]["x"], 45.0);
    assert_eq!(section["startPoint"]["x"], 62.0);
    assert_eq!(section["endPoint"]["x"], 62.0);
}
