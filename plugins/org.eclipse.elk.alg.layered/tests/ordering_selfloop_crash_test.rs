mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPSILON: f64 = 0.1;

#[test]
fn ordering_selfloop_crash_matches_java() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/ordering/selfloop_crash.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("selfloop_crash resource should load");

    run_layout(&graph);

    let n3 = find_node_by_identifier(&graph, "n3").expect("n3 should exist");
    let n4 = find_node_by_identifier(&graph, "n4").expect("n4 should exist");

    let n3_y = node_y(&n3);
    let n4_y = node_y(&n4);

    assert!(
        n4_y + EPSILON < n3_y,
        "expected n4 above n3 (n4_y={n4_y}, n3_y={n3_y})"
    );
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}
