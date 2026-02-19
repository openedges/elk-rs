mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};

#[test]
fn issue_425_self_loop_in_compound_matches_java_reference_sizes() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_425_self_loop_in_compound.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_425 resource should load");

    run_recursive_layout(&graph);

    let node_1 = find_node_by_identifier(&graph, "Node_1").expect("Node_1 should exist");
    let node_1_width = node_1.borrow_mut().connectable().shape().width();

    assert!(
        (node_1_width - 30.0).abs() <= 1e-6,
        "Node_1 width should match Java (30.0), got {node_1_width}"
    );
}
