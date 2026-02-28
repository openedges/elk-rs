
use crate::common::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout};

const COORDINATE_FUZZINESS: f64 = 0.5;

#[test]
fn issue_476_multiple_node_labels_are_centered_in_node_width() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_476_node_labels_center.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_476 resource should load");

    run_layout(&graph);

    let n1 = find_node_by_identifier(&graph, "n1").expect("n1 node should exist");
    let n2 = find_node_by_identifier(&graph, "n2").expect("n2 node should exist");
    for node in [n1, n2] {
        assert_node_labels_centered(&node);
    }
}

fn assert_node_labels_centered(node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef) {
    let (node_width, labels) = {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        (
            shape.width(),
            shape
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        )
    };

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;

    for label in labels {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        min_x = min_x.min(shape.x());
        max_x = max_x.max(shape.x());
        assert!(
            shape.x().is_finite()
                && shape.y().is_finite()
                && shape.x() >= 0.0
                && shape.x() + shape.width() <= node_width + COORDINATE_FUZZINESS,
            "node label has invalid horizontal placement: x={}, width={}, node_width={node_width}",
            shape.x(),
            shape.width()
        );
    }

    assert!(
        (max_x - min_x).abs() <= COORDINATE_FUZZINESS,
        "node labels are not horizontally aligned: min_x={min_x}, max_x={max_x}"
    );
}
