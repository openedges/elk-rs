mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdgeRef, ElkNodeRef};

#[test]
fn issue_734_node_and_edge_centers_stay_aligned() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_734_center_alignment.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_734 resource should load");

    run_layout(&graph);

    let node_span = compute_node_x_span(&graph);
    let edge_span = compute_edge_x_span(&graph);

    let node_center = (node_span.0 + node_span.1) / 2.0;
    let edge_center = (edge_span.0 + edge_span.1) / 2.0;

    assert!(
        (node_center - edge_center).abs() <= 0.5,
        "node/edge center mismatch: node_center={node_center}, edge_center={edge_center}"
    );
}

fn compute_node_x_span(graph: &ElkNodeRef) -> (f64, f64) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for node in children {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        min_x = min_x.min(shape.x());
        max_x = max_x.max(shape.x() + shape.width());
    }
    (min_x, max_x)
}

fn compute_edge_x_span(graph: &ElkNodeRef) -> (f64, f64) {
    let edges: Vec<ElkEdgeRef> = graph.borrow_mut().contained_edges().iter().cloned().collect();

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;

    for edge in edges {
        let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
        for section in sections {
            let chain = ElkUtil::create_vector_chain(&section);
            for idx in 0..chain.size() {
                let point = chain.get(idx);
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
            }
        }
    }

    (min_x, max_x)
}
