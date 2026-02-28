
use crate::common::elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout};

#[test]
fn issue_445_feedback_edge_label_is_not_left_at_origin() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_445_feedback_edge_label.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_445 resource should load");

    run_layout(&graph);

    let forward =
        find_edge_by_identifier(&graph, "n1", "n2").expect("forward edge n1->n2 should exist");
    let feedback =
        find_edge_by_identifier(&graph, "n2", "n1").expect("feedback edge n2->n1 should exist");

    let labels = feedback
        .borrow_mut()
        .element()
        .labels()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 1, "expected one feedback edge label");

    let mut label_mut = labels[0].borrow_mut();
    let shape = label_mut.shape();
    assert!(
        shape.x().is_finite()
            && shape.y().is_finite()
            && shape.width() > 0.0
            && shape.height() > 0.0,
        "feedback label has invalid geometry: ({}, {}, {}, {})",
        shape.x(),
        shape.y(),
        shape.width(),
        shape.height()
    );

    assert!(!forward.borrow_mut().sections().is_empty());
}
