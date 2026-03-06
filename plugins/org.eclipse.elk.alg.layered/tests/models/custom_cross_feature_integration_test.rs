use crate::common::issue_support::{
    create_edge, create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

fn set_edge_property<T: Clone + Send + Sync + 'static>(
    edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: T,
) {
    let mut edge_mut = edge.borrow_mut();
    edge_mut
        .element()
        .properties_mut()
        .set_property(property, Some(value));
}

#[test]
fn in_layer_edge_with_port_constraints_valid_output() {
    // Cross-feature test: ignoreEdgeInLayer (Feature A) + FixedSide ports (Feature B)
    // EAST→WEST in-layer edges with FixedSide ports should produce valid layout
    init_layered_options();

    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let n1 = create_node(&graph, 30.0, 30.0);
    set_node_property(&n1, LayeredOptions::PORT_CONSTRAINTS, PortConstraints::FixedSide);
    let n2 = create_node(&graph, 30.0, 30.0);
    set_node_property(&n2, LayeredOptions::PORT_CONSTRAINTS, PortConstraints::FixedSide);
    let n3 = create_node(&graph, 30.0, 30.0);

    // Normal edge n3 -> n1 (puts n3 in different layer from n1)
    let _e1 = create_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n1.clone()),
    );

    // ignoreEdgeInLayer edge n1 -> n2 (same layer) via EAST→WEST ports
    let p1 = ElkGraphUtil::create_port(Some(n1.clone()));
    {
        let mut p = p1.borrow_mut();
        p.connectable().shape().set_dimensions(5.0, 5.0);
        p.connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions::PORT_SIDE, Some(PortSide::East));
    }
    let p2 = ElkGraphUtil::create_port(Some(n2.clone()));
    {
        let mut p = p2.borrow_mut();
        p.connectable().shape().set_dimensions(5.0, 5.0);
        p.connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions::PORT_SIDE, Some(PortSide::West));
    }

    let edge2 = create_edge(
        ElkConnectableShapeRef::Port(p1.clone()),
        ElkConnectableShapeRef::Port(p2.clone()),
    );
    set_edge_property(&edge2, LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER, true);

    run_layout(&graph);

    // All coordinates should be finite and non-negative (pipeline completed successfully)
    for (name, node) in [("n1", &n1), ("n2", &n2), ("n3", &n3)] {
        let x = node.borrow_mut().connectable().shape().x();
        let y = node.borrow_mut().connectable().shape().y();
        assert!(x.is_finite() && x >= 0.0, "{name} x invalid: {x}");
        assert!(y.is_finite() && y >= 0.0, "{name} y invalid: {y}");
    }

    // n1 and n2 (ignoreEdgeInLayer) should be much closer than n3 is to either
    let n1_x = n1.borrow_mut().connectable().shape().x();
    let n2_x = n2.borrow_mut().connectable().shape().x();
    let n3_x = n3.borrow_mut().connectable().shape().x();
    let n1_n2_dist = (n1_x - n2_x).abs();
    let n3_n1_dist = (n3_x - n1_x).abs();
    assert!(
        n3_n1_dist > n1_n2_dist,
        "n3 should be farther from n1 than n2 is: n1_n2={n1_n2_dist}, n3_n1={n3_n1_dist}"
    );
}
