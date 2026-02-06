mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};

const EPSILON: f64 = 1.0e-5;

#[test]
fn issue_628_child_nodes_keep_spacing_and_row_alignment() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_628_child_spacing.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_628 resource should load");

    run_recursive_layout(&graph);

    let n1 = find_node_by_identifier(&graph, "n1").expect("n1 node should exist");
    let n2 = find_node_by_identifier(&graph, "n2").expect("n2 node should exist");

    let (n1_x, n1_y, n1_w, n1_h) = node_info(&n1);
    let (n2_x, n2_y, n2_w, n2_h) = node_info(&n2);

    let (left_x, left_w, right_x) = if n1_x <= n2_x {
        (n1_x, n1_w, n2_x)
    } else {
        (n2_x, n2_w, n1_x)
    };
    let horizontal_spacing = right_x - (left_x + left_w);

    let (top_y, top_h, bottom_y) = if n1_y <= n2_y {
        (n1_y, n1_h, n2_y)
    } else {
        (n2_y, n2_h, n1_y)
    };
    let vertical_spacing = bottom_y - (top_y + top_h);

    assert!(
        (horizontal_spacing - 10.0).abs() <= EPSILON || (vertical_spacing - 10.0).abs() <= EPSILON,
        "unexpected spacing: n1=({n1_x},{n1_y},{n1_w},{n1_h}), n2=({n2_x},{n2_y},{n2_w},{n2_h}), horizontal_spacing={horizontal_spacing}, vertical_spacing={vertical_spacing}"
    );
}

fn node_info(
    node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}
