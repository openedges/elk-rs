mod issue_support;

use issue_support::{
    init_layered_options, run_recursive_layout, set_node_property, set_port_property,
};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkNodeRef, ElkPortRef,
};

#[test]
fn plain_layout_recursive_hierarchical_smoke() {
    init_layered_options();

    let graph = ElkGraphUtil::create_graph();
    create_simple_compound_graph(&graph);
    create_simple_compound_graph(&graph);

    for node in all_nodes(&graph) {
        if has_children(&node) {
            set_node_property(
                &node,
                CoreOptions::ALGORITHM,
                LayeredOptions::ALGORITHM_ID.to_string(),
            );
        }
    }

    run_recursive_layout(&graph);

    let leaves: Vec<ElkNodeRef> = all_nodes(&graph)
        .into_iter()
        .filter(|node| !has_children(node))
        .collect();
    assert!(!leaves.is_empty(), "expected non-empty leaf node set");

    let mut moved_leaf = false;
    for leaf in leaves {
        let mut leaf_guard = leaf.borrow_mut();
        let shape = leaf_guard.connectable().shape();
        assert!(shape.x().is_finite(), "leaf x should be finite");
        assert!(shape.y().is_finite(), "leaf y should be finite");
        if shape.x() > 0.0 || shape.y() > 0.0 {
            moved_leaf = true;
        }
    }
    assert!(
        moved_leaf,
        "at least one leaf node should be moved away from the origin"
    );
}

#[test]
fn direct_layout_layered_provider_sets_positive_coordinates() {
    init_layered_options();

    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, LayeredOptions::DIRECTION, Direction::Right);

    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));
    configure_direct_layout_node(&node1);
    configure_direct_layout_node(&node2);

    let port1 = ElkGraphUtil::create_port(Some(node1.clone()));
    let port2 = ElkGraphUtil::create_port(Some(node2.clone()));
    configure_port(&port1, PortSide::East, 30.0, 15.0);
    configure_port(&port2, PortSide::West, 0.0, 15.0);
    let _edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port1),
        ElkConnectableShapeRef::Port(port2),
    );

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(&graph, &mut BasicProgressMonitor::new());

    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for child in children {
        let mut child_guard = child.borrow_mut();
        let shape = child_guard.connectable().shape();
        assert!(shape.x() > 0.0, "child x should be positive after layout");
        assert!(shape.y() > 0.0, "child y should be positive after layout");
    }
}

fn create_simple_compound_graph(parent: &ElkNodeRef) {
    let compound = ElkGraphUtil::create_node(Some(parent.clone()));
    set_dimensions(&compound, 120.0, 80.0);

    let node1 = ElkGraphUtil::create_node(Some(compound.clone()));
    let node2 = ElkGraphUtil::create_node(Some(compound));
    configure_direct_layout_node(&node1);
    configure_direct_layout_node(&node2);

    let port1 = ElkGraphUtil::create_port(Some(node1.clone()));
    let port2 = ElkGraphUtil::create_port(Some(node2.clone()));
    configure_port(&port1, PortSide::East, 30.0, 15.0);
    configure_port(&port2, PortSide::West, 0.0, 15.0);
    let _edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port1),
        ElkConnectableShapeRef::Port(port2),
    );
}

fn configure_direct_layout_node(node: &ElkNodeRef) {
    set_dimensions(node, 30.0, 30.0);
    set_node_property(
        node,
        LayeredOptions::NODE_SIZE_CONSTRAINTS,
        SizeConstraint::fixed(),
    );
    set_node_property(
        node,
        LayeredOptions::PORT_CONSTRAINTS,
        PortConstraints::FixedPos,
    );
}

fn configure_port(port: &ElkPortRef, side: PortSide, x: f64, y: f64) {
    {
        let mut port_guard = port.borrow_mut();
        port_guard.connectable().shape().set_location(x, y);
    }
    set_port_property(port, LayeredOptions::PORT_SIDE, side);
    set_port_property(port, CoreOptions::PORT_SIDE, side);
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_guard = node.borrow_mut();
    node_guard
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn all_nodes(root: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut nodes = vec![root.clone()];
    let mut index = 0usize;
    while index < nodes.len() {
        let children: Vec<ElkNodeRef> = nodes[index]
            .borrow_mut()
            .children()
            .iter()
            .cloned()
            .collect();
        nodes.extend(children);
        index += 1;
    }
    nodes
}

fn has_children(node: &ElkNodeRef) -> bool {
    !node.borrow_mut().children().is_empty()
}
