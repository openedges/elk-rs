mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_edge_by_identifier, find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMath, ElkRectangle};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdgeRef, ElkNodeRef};

#[test]
fn issue_515_edge_segments_do_not_cross_unrelated_nodes() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_515_edge_avoidance.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_515 resource should load");

    run_layout(&graph);

    let b = find_node_by_identifier(&graph, "b").expect("node b should exist");
    let c = find_node_by_identifier(&graph, "c").expect("node c should exist");
    let d = find_node_by_identifier(&graph, "d").expect("node d should exist");

    let e1 = find_edge_by_identifier(&graph, "a", "b").expect("edge a->b should exist");
    let e2 = find_edge_by_identifier(&graph, "b", "c").expect("edge b->c should exist");

    assert_edge_avoids_non_endpoint_nodes(&e1, &[b.clone(), c.clone(), d.clone()]);
    assert_edge_avoids_non_endpoint_nodes(&e2, &[c, d, b]);
}

fn assert_edge_avoids_non_endpoint_nodes(edge: &ElkEdgeRef, excluded: &[ElkNodeRef]) {
    let sections = edge.borrow_mut().sections().iter().cloned().collect::<Vec<_>>();
    let node_boxes = excluded.iter().map(node_rect).collect::<Vec<_>>();

    for section in sections {
        let chain = ElkUtil::create_vector_chain(&section);
        if chain.size() < 2 {
            continue;
        }

        let mut start = chain.get(0);
        for idx in 1..chain.size() {
            let end = chain.get(idx);
            for box_rect in &node_boxes {
                assert!(
                    !ElkMath::intersects((box_rect, &start, &end)),
                    "edge segment intersects unrelated node: start={start:?}, end={end:?}, box={box_rect:?}"
                );
            }
            start = end;
        }
    }
}

fn node_rect(node: &ElkNodeRef) -> ElkRectangle {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    ElkRectangle::with_values(shape.x(), shape.y(), shape.width(), shape.height())
}
