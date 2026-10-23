//! Java reference cases for disconnected-component compaction.
//! See tests/component_compaction.md for reference provenance and the Java fix.

use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
use serde_json::Value;
use std::path::PathBuf;

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

fn check_case(name: &str) {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/component_compaction");
    let input = std::fs::read_to_string(directory.join(format!("{name}.input.json"))).unwrap();
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join(format!("{name}.expected.json"))).unwrap(),
    )
    .unwrap();
    let actual: Value =
        serde_json::from_str(&layout_api::layout_json(&input, "{}").unwrap()).unwrap();
    assert_layout_equal(&expected, &actual, name);
}

macro_rules! java_reference_tests {
    ($($name:ident),* $(,)?) => { $(
        #[test]
        fn $name() { check_case(stringify!($name)); }
    )* };
}

java_reference_tests!(
    self_loop,
    zero_size_components,
    disconnected_right,
    disconnected_left,
    disconnected_down,
    disconnected_up,
    labels_and_spacing,
    external_north,
    external_east,
    external_south,
    external_west,
    external_all_sides,
    unconnected_external_ports,
    disabled,
    ticket_213_zero_size,
    ticket_213_positive_size,
);

#[test]
fn empty_and_single_component_remain_valid() {
    for children in ["[]", r#"[{"id":"single","width":0,"height":0}]"#] {
        let input = format!(
            r#"{{"id":"root","layoutOptions":{{"elk.algorithm":"layered","elk.layered.compaction.connectedComponents":"true"}},"children":{children}}}"#
        );
        let result: Value =
            serde_json::from_str(&layout_api::layout_json(&input, "{}").unwrap()).unwrap();
        assert!(result["width"].as_f64().unwrap().is_finite());
        assert!(result["height"].as_f64().unwrap().is_finite());
        assert_eq!(
            result["children"].as_array().unwrap().len(),
            serde_json::from_str::<Value>(children)
                .unwrap()
                .as_array()
                .unwrap()
                .len()
        );
    }
}
