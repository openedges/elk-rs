
use crate::common::elkt_test_loader::load_layered_graph_from_elkt;
use crate::common::issue_support::{init_layered_options, run_layout};

#[test]
fn issue_444_self_loop_layout_does_not_panic() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_444_self_loop.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_444 resource should load");

    run_layout(&graph);

    let edge_sections = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .map(|edge| edge.borrow_mut().sections().len())
        .sum::<usize>();
    assert!(
        edge_sections > 0,
        "self-loop edge should produce at least one section"
    );
}
