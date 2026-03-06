use crate::common::issue_support::{
    create_edge, create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayerConstraint, LayeredOptions,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

#[test]
fn first_separate_last_separate_full_pipeline_no_panic() {
    init_layered_options();

    let graph = create_graph();

    let first_sep = create_node(&graph, 30.0, 30.0);
    set_node_property(
        &first_sep,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        LayerConstraint::FirstSeparate,
    );

    let last_sep = create_node(&graph, 30.0, 30.0);
    set_node_property(
        &last_sep,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        LayerConstraint::LastSeparate,
    );

    let middle = create_node(&graph, 30.0, 30.0);

    let _e1 = create_edge(
        ElkConnectableShapeRef::Node(first_sep.clone()),
        ElkConnectableShapeRef::Node(middle.clone()),
    );
    let _e2 = create_edge(
        ElkConnectableShapeRef::Node(middle.clone()),
        ElkConnectableShapeRef::Node(last_sep.clone()),
    );

    // Should not panic through the full pipeline
    run_layout(&graph);

    // FirstSeparate should be leftmost, LastSeparate rightmost
    let fs_x = first_sep.borrow_mut().connectable().shape().x();
    let mid_x = middle.borrow_mut().connectable().shape().x();
    let ls_x = last_sep.borrow_mut().connectable().shape().x();

    assert!(
        fs_x < mid_x,
        "FirstSeparate should be left of middle: fs_x={fs_x}, mid_x={mid_x}"
    );
    assert!(
        mid_x < ls_x,
        "middle should be left of LastSeparate: mid_x={mid_x}, ls_x={ls_x}"
    );
}

#[test]
fn first_separate_with_incoming_edge_no_panic() {
    // B-2: ensure_no_inacceptable_edges disabled — incoming edge to FIRST_SEPARATE
    // should not panic in the full pipeline
    init_layered_options();

    let graph = create_graph();

    let normal = create_node(&graph, 30.0, 30.0);
    let first_sep = create_node(&graph, 30.0, 30.0);
    set_node_property(
        &first_sep,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        LayerConstraint::FirstSeparate,
    );

    // incoming edge TO first_separate (would have panicked before B-2 fix)
    let _edge = create_edge(
        ElkConnectableShapeRef::Node(normal.clone()),
        ElkConnectableShapeRef::Node(first_sep.clone()),
    );

    // Should complete without panic
    run_layout(&graph);

    // Coordinates should be valid
    for (name, node) in [("normal", &normal), ("first_sep", &first_sep)] {
        let x = node.borrow_mut().connectable().shape().x();
        let y = node.borrow_mut().connectable().shape().y();
        assert!(
            x.is_finite() && y.is_finite(),
            "{name} should have finite coordinates: x={x}, y={y}"
        );
    }
}

#[test]
fn last_separate_with_outgoing_edge_no_panic() {
    // B-2: ensure_no_inacceptable_edges disabled — outgoing edge from LAST_SEPARATE
    // should not panic in the full pipeline
    init_layered_options();

    let graph = create_graph();

    let last_sep = create_node(&graph, 30.0, 30.0);
    set_node_property(
        &last_sep,
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        LayerConstraint::LastSeparate,
    );
    let normal = create_node(&graph, 30.0, 30.0);

    // outgoing edge FROM last_separate (would have panicked before B-2 fix)
    let _edge = create_edge(
        ElkConnectableShapeRef::Node(last_sep.clone()),
        ElkConnectableShapeRef::Node(normal.clone()),
    );

    // Should complete without panic
    run_layout(&graph);

    for (name, node) in [("last_sep", &last_sep), ("normal", &normal)] {
        let x = node.borrow_mut().connectable().shape().x();
        let y = node.borrow_mut().connectable().shape().y();
        assert!(
            x.is_finite() && y.is_finite(),
            "{name} should have finite coordinates: x={x}, y={y}"
        );
    }
}
