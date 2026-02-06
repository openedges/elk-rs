mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_edge_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkPortRef};

const TOLERANCE: f64 = 0.5;

#[test]
fn issue_546_edge_start_is_close_to_source_port_anchor() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_546_port_anchor.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_546 resource should load");

    run_layout(&graph);

    let left_port = find_port_by_identifier(&graph, "p_left").expect("left port should exist");
    let edge = find_edge_by_identifier(&graph, "p_left", "p_right")
        .expect("edge p_left->p_right should exist");

    let (port_x, port_y, port_w, port_h) = absolute_port_rect(&left_port);
    let first_section = edge
        .borrow_mut()
        .sections()
        .iter()
        .next()
        .cloned()
        .expect("expected routed section");

    let chain = ElkUtil::create_vector_chain(&first_section);
    let start = chain.get(0);

    assert!(
        start.x >= port_x - TOLERANCE
            && start.x <= port_x + port_w + TOLERANCE
            && start.y >= port_y - TOLERANCE
            && start.y <= port_y + port_h + TOLERANCE,
        "edge start is not near source port: start={start:?}, port=({}, {}, {}, {})",
        port_x,
        port_y,
        port_w,
        port_h
    );
}

fn absolute_port_rect(port: &ElkPortRef) -> (f64, f64, f64, f64) {
    let position = ElkUtil::absolute_position(&ElkGraphElementRef::Port(port.clone()))
        .expect("port should have absolute position");
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    (position.x, position.y, shape.width(), shape.height())
}
