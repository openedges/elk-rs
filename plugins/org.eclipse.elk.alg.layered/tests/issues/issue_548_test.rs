
use crate::common::elkt_test_loader::load_layered_graph_from_elkt;
use crate::common::issue_support::{init_layered_options, run_layout};

#[test]
fn issue_548_inside_self_loop_case_does_not_panic() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_548_inside_self_loops.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_548 resource should load");

    run_layout(&graph);

    let all_edges_have_sections = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .all(|edge| !edge.borrow_mut().sections().is_empty());
    assert!(
        all_edges_have_sections,
        "expected routed sections for self-loop edges"
    );
}
