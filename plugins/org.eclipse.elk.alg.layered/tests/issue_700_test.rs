mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_layout};

#[test]
fn issue_700_edges_are_routed_with_non_zero_section_coordinates() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_700_non_zero_sections.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_700 resource should load");

    run_layout(&graph);

    let edges = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(!edges.is_empty(), "graph should contain at least one edge");

    for edge in edges {
        let sections = edge.borrow_mut().sections().iter().cloned().collect::<Vec<_>>();
        assert_eq!(sections.len(), 1, "edge should have exactly one section");

        let section = sections[0].borrow();
        let has_non_zero_coordinates = section.start_x() != 0.0
            || section.start_y() != 0.0
            || section.end_x() != 0.0
            || section.end_y() != 0.0;
        assert!(
            has_non_zero_coordinates,
            "edge section should not be at the origin"
        );
    }
}
