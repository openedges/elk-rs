mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef;

const TOLERANCE: f64 = 0.05;

#[test]
fn issue_726_orthogonal_routing_keeps_axis_aligned_segments() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_726_orthogonal_segments.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_726 resource should load");

    run_layout(&graph);

    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();

    for edge in edges {
        let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
        for section in sections {
            let chain = ElkUtil::create_vector_chain(&section);
            assert!(chain.size() >= 2, "edge route must have at least two points");

            let mut previous = chain.get(0);
            for idx in 1..chain.size() {
                let current = chain.get(idx);
                let horizontal = (previous.y - current.y).abs() <= TOLERANCE;
                let vertical = (previous.x - current.x).abs() <= TOLERANCE;
                assert!(
                    horizontal || vertical,
                    "non-orthogonal segment found: prev={previous:?}, current={current:?}"
                );
                previous = current;
            }
        }
    }
}
