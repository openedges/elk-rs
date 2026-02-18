mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;

#[test]
fn issue_502_compound_nodes_are_large_enough_for_children_and_routes() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_502_compound_sizing.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_502 resource should load");

    run_recursive_layout(&graph);

    let compound = find_node_by_identifier(&graph, "compound").expect("compound node should exist");
    assert_node_large_enough_for_content(&compound);
}

fn assert_node_large_enough_for_content(
    node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
) {
    let (node_width, node_height, children, edges) = {
        let mut node_mut = node.borrow_mut();
        (
            node_mut.connectable().shape().width(),
            node_mut.connectable().shape().height(),
            node_mut.children().iter().cloned().collect::<Vec<_>>(),
            node_mut
                .contained_edges()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        )
    };

    let mut required_bounds = ElkRectangle::default();

    for child in children {
        let mut child_mut = child.borrow_mut();
        let shape = child_mut.connectable().shape();
        required_bounds.width = required_bounds.width.max(shape.x() + shape.width());
        required_bounds.height = required_bounds.height.max(shape.y() + shape.height());
    }

    for edge in edges {
        let sections = edge
            .borrow_mut()
            .sections()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for section in sections {
            let chain = ElkUtil::create_vector_chain(&section);
            for idx in 0..chain.size() {
                let point = chain.get(idx);
                required_bounds.width = required_bounds.width.max(point.x);
                required_bounds.height = required_bounds.height.max(point.y);
            }
        }
    }

    assert!(
        node_width + 0.5 >= required_bounds.width && node_height + 0.5 >= required_bounds.height,
        "node not large enough: required=({}, {}), actual=({}, {})",
        required_bounds.width,
        required_bounds.height,
        node_width,
        node_height
    );
}
