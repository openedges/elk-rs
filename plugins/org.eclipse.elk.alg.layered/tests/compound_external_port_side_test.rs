mod issue_support;

use issue_support::{
    create_edge, create_graph, create_node, create_port, init_layered_options, node_bounds,
    run_recursive_layout, set_node_property, set_port_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

#[test]
fn compound_external_port_preserves_side_and_position() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::HIERARCHY_HANDLING,
        HierarchyHandling::IncludeChildren,
    );

    let parent = create_node(&graph, 100.0, 80.0);
    set_node_property(
        &parent,
        LayeredOptions::PORT_CONSTRAINTS,
        PortConstraints::Free,
    );
    let parent_port = create_port(&parent, 8.0, 8.0);
    set_port_property(&parent_port, LayeredOptions::PORT_SIDE, PortSide::East);

    let child = create_node(&parent, 30.0, 20.0);
    let child_port = create_port(&child, 8.0, 8.0);
    create_edge(
        ElkConnectableShapeRef::Port(child_port),
        ElkConnectableShapeRef::Port(parent_port.clone()),
    );

    run_recursive_layout(&graph);

    let (_, _, node_width, _) = node_bounds(&parent);
    let (port_x, port_side) = {
        let mut port_mut = parent_port.borrow_mut();
        let shape = port_mut.connectable().shape();
        let side = shape
            .graph_element()
            .properties_mut()
            .get_property(LayeredOptions::PORT_SIDE)
            .unwrap_or(PortSide::Undefined);
        (shape.x(), side)
    };

    assert_eq!(port_side, PortSide::East, "external port side should stay EAST");
    assert!(
        port_x >= node_width - 1e-6,
        "expected EAST port on or beyond node width, got x={port_x} width={node_width}"
    );
}
