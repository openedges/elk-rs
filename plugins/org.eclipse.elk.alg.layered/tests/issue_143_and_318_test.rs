mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdgeRef, ElkNodeRef};

const SEGMENT_TOLERANCE: f64 = 0.01;
const PROXIMITY_THRESHOLD: f64 = 1.0;

#[test]
fn issue_143_and_318_horizontal_segments_keep_vertical_distance() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_143_318_horizontal_segments.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_143_318 resource should load");

    run_layout(&graph);

    let segments = gather_horizontal_segments(&graph);
    assert!(
        !segments.is_empty(),
        "expected horizontal edge segments after orthogonal routing"
    );

    for (index_a, (edge_a, x1_a, x2_a, y_a)) in segments.iter().enumerate() {
        for (edge_b, x1_b, x2_b, y_b) in segments.iter().skip(index_a + 1) {
            if edge_a == edge_b {
                continue;
            }
            if segments_overlap_horizontally(*x1_a, *x2_a, *x1_b, *x2_b) {
                assert!(
                    (y_a - y_b).abs() >= PROXIMITY_THRESHOLD,
                    "horizontal segments too close: edge {edge_a} ({x1_a},{x2_a},{y_a}) edge {edge_b} ({x1_b},{x2_b},{y_b})"
                );
            }
        }
    }
}

fn gather_horizontal_segments(graph: &ElkNodeRef) -> Vec<(usize, f64, f64, f64)> {
    let edges: Vec<ElkEdgeRef> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();
    let mut segments = Vec::new();

    for (edge_index, edge) in edges.into_iter().enumerate() {
        let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
        for section in sections {
            let chain = ElkUtil::create_vector_chain(&section);
            if chain.size() < 2 {
                continue;
            }

            let mut previous = chain.get(0);
            for idx in 1..chain.size() {
                let current = chain.get(idx);
                if (previous.y - current.y).abs() <= SEGMENT_TOLERANCE {
                    let x1 = previous.x.min(current.x);
                    let x2 = previous.x.max(current.x);
                    segments.push((edge_index, x1, x2, previous.y));
                }
                previous = current;
            }
        }
    }

    segments
}

fn segments_overlap_horizontally(a_start: f64, a_end: f64, b_start: f64, b_end: f64) -> bool {
    a_start < b_end && b_start < a_end
}
