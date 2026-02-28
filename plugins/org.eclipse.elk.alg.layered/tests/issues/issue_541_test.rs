
use crate::common::elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout};

#[test]
fn issue_541_end_label_sorting_case_does_not_panic() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_541_end_label_sorting.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_541 resource should load");

    run_layout(&graph);

    let edge = find_edge_by_identifier(&graph, "left", "right")
        .expect("main edge left->right should exist");
    let label_count = edge.borrow_mut().element().labels().len();
    assert_eq!(label_count, 2, "main edge should keep two labels");

    assert!(!edge.borrow_mut().sections().is_empty());
}
