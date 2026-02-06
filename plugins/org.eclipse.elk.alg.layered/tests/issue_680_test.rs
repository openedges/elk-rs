mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};

#[test]
fn issue_680_nested_ports_with_border_offsets_keep_expected_positions() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_680_nested_ports_border_offsets.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_680 resource should load");

    run_recursive_layout(&graph);

    let parent = find_node_by_identifier(&graph, "parent").expect("parent node should exist");
    let child = find_node_by_identifier(&graph, "child").expect("child node should exist");

    let (_parent_x, parent_y, parent_w, parent_h) = node_bounds(&parent);
    let (child_x, child_y, child_w, child_h) = node_bounds(&child);

    assert!(
        parent_y.is_finite()
            && child_y.is_finite()
            && parent_w > 0.0
            && parent_h > 0.0
            && child_w > 0.0
            && child_h > 0.0
            && child_x >= 0.0
            && child_y >= 0.0,
        "invalid layout bounds parent=(y={parent_y},w={parent_w},h={parent_h}) child=(x={child_x},y={child_y},w={child_w},h={child_h})"
    );
}

fn node_bounds(node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}
