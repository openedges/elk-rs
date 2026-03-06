use crate::common::issue_support::{
    create_edge, create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredOptions, LayeringStrategy,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

const EPSILON: f64 = 1.0e-4;

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
fn ignore_edge_in_layer_full_pipeline_same_layer() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let n1 = create_node(&graph, 30.0, 30.0);
    let n2 = create_node(&graph, 30.0, 30.0);

    let edge = create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    set_edge_property(&edge, LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER, true);

    run_layout(&graph);

    let n1_x = n1.borrow_mut().connectable().shape().x();
    let n2_x = n2.borrow_mut().connectable().shape().x();

    assert!(
        (n1_x - n2_x).abs() < EPSILON,
        "ignoreEdgeInLayer nodes should be in the same layer (same x), got n1_x={n1_x}, n2_x={n2_x}"
    );
}

#[test]
fn ignore_edge_in_layer_false_different_layers() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let n1 = create_node(&graph, 30.0, 30.0);
    let n2 = create_node(&graph, 30.0, 30.0);

    let _edge = create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    let n1_x = n1.borrow_mut().connectable().shape().x();
    let n2_x = n2.borrow_mut().connectable().shape().x();

    assert!(
        (n1_x - n2_x).abs() > EPSILON,
        "normal edge nodes should be in different layers (different x), got n1_x={n1_x}, n2_x={n2_x}"
    );
}

#[test]
fn ignore_edge_in_layer_valid_edge_routing() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::LAYERING_STRATEGY,
        LayeringStrategy::NetworkSimplex,
    );

    let n1 = create_node(&graph, 30.0, 30.0);
    let n2 = create_node(&graph, 30.0, 30.0);
    let n3 = create_node(&graph, 30.0, 30.0);

    // n1 -> n2 (normal), n2 -> n3 (ignore), so n2 and n3 same layer, n1 different
    let _edge1 = create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let edge2 = create_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    set_edge_property(&edge2, LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER, true);

    run_layout(&graph);

    let n1_x = n1.borrow_mut().connectable().shape().x();
    let n2_x = n2.borrow_mut().connectable().shape().x();
    let n3_x = n3.borrow_mut().connectable().shape().x();

    assert!(
        (n1_x - n2_x).abs() > EPSILON,
        "n1 and n2 should be in different layers"
    );
    assert!(
        (n2_x - n3_x).abs() < EPSILON,
        "n2 and n3 should be in the same layer (ignoreEdgeInLayer)"
    );

    // All coordinates should be valid (finite, non-negative)
    for (name, node) in [("n1", &n1), ("n2", &n2), ("n3", &n3)] {
        let x = node.borrow_mut().connectable().shape().x();
        let y = node.borrow_mut().connectable().shape().y();
        assert!(x.is_finite() && x >= 0.0, "{name} x should be valid: {x}");
        assert!(y.is_finite() && y >= 0.0, "{name} y should be valid: {y}");
    }

    // All edge sections should have finite coordinates
    let edges: Vec<_> = graph
        .borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect();
    for edge in &edges {
        let sections: Vec<_> = edge.borrow_mut().sections().iter().cloned().collect();
        for section in &sections {
            let s = section.borrow();
            assert!(s.start_x().is_finite(), "edge section start_x not finite");
            assert!(s.start_y().is_finite(), "edge section start_y not finite");
            assert!(s.end_x().is_finite(), "edge section end_x not finite");
            assert!(s.end_y().is_finite(), "edge section end_y not finite");
        }
    }
}
