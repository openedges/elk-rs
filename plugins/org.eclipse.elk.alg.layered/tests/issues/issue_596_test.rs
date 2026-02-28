
use crate::common::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_recursive_layout};

const TOLERANCE: f64 = 0.5;

#[test]
fn issue_596_single_child_is_centered_in_hierarchical_node() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_596_single_child_centering.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_596 resource should load");

    run_recursive_layout(&graph);

    let outer = find_node_by_identifier(&graph, "outer").expect("outer node should exist");
    let inner = find_node_by_identifier(&graph, "inner").expect("inner node should exist");

    let (outer_w, inner_w, inner_x) = {
        let mut outer_mut = outer.borrow_mut();
        let outer_shape = outer_mut.connectable().shape();
        let outer_w = outer_shape.width();

        let mut inner_mut = inner.borrow_mut();
        let inner_shape = inner_mut.connectable().shape();
        (outer_w, inner_shape.width(), inner_shape.x())
    };

    let expected = (outer_w - inner_w) / 2.0;
    assert!(
        (inner_x - expected).abs() <= TOLERANCE,
        "inner node not centered: inner_x={inner_x}, expected={expected}, outer_w={outer_w}, inner_w={inner_w}"
    );
}
