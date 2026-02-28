
use crate::common::elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_recursive_layout};

fn assert_edge_has_sections(
    edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef,
    label: &str,
) {
    let section_len = edge.borrow_mut().sections().len();
    assert!(
        section_len > 0,
        "{label}: expected edge sections to be present after layout"
    );
}

#[test]
fn hierarchy_center_edge_label_problem_writes_sections() {
    init_layered_options();

    let path = format!(
        "{}/../../external/elk-models/tests/layered/edge_label_placement/hierarchy_center_edge_label_problem.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .expect("hierarchy_center_edge_label_problem should load");

    run_recursive_layout(&graph);

    let edge = find_edge_by_identifier(&graph, "EinAndererKnoten", "EinZweiterKnoten")
        .expect("edge EinAndererKnoten -> EinZweiterKnoten should exist");
    assert_edge_has_sections(&edge, "hierarchy_center_edge_label_problem");
}

#[test]
fn include_children_fixed_order_writes_sections() {
    init_layered_options();

    let path = format!(
        "{}/../../external/elk-models/tickets/layered/040_includeChildrenWithFixedOrder.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph =
        load_layered_graph_from_elkt(&path).expect("040_includeChildrenWithFixedOrder should load");

    run_recursive_layout(&graph);

    let edge = find_edge_by_identifier(&graph, "n2", "n1").expect("edge n2 -> n1 should exist");
    assert_edge_has_sections(&edge, "040_includeChildrenWithFixedOrder");
}
