
use crate::common::elkt_test_loader::load_layered_graph_from_elkt;
use crate::common::issue_support::{init_layered_options, run_layout};

#[test]
fn issue_870_network_simplex_node_placer_does_not_fail() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_870_network_simplex.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_870 resource should load");

    run_layout(&graph);

    let (graph_w, graph_h) = {
        let mut graph_mut = graph.borrow_mut();
        let shape = graph_mut.connectable().shape();
        (shape.width(), shape.height())
    };
    assert!(
        graph_w.is_finite() && graph_h.is_finite() && graph_w >= 0.0 && graph_h >= 0.0,
        "invalid graph size after layout: ({graph_w}, {graph_h})"
    );
}
