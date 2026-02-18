mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_recursive_layout};

#[test]
fn issue_665_hierarchical_graph_layout_does_not_fail_import() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_665_hierarchical_import.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_665 resource should load");

    run_recursive_layout(&graph);

    let (graph_width, graph_height) = {
        let mut graph_mut = graph.borrow_mut();
        let shape = graph_mut.connectable().shape();
        (shape.width(), shape.height())
    };
    assert!(
        graph_width.is_finite()
            && graph_height.is_finite()
            && graph_width >= 0.0
            && graph_height >= 0.0,
        "invalid graph size after layout (w={graph_width}, h={graph_height})"
    );

    let edges = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !edges.is_empty(),
        "expected at least one edge after loading"
    );

    for edge in edges {
        assert!(
            edge.borrow().is_connected(),
            "edge lost connectivity after layout"
        );
    }
}
