
use crate::common::issue_support::{
    create_edge, create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    ConstraintCalculationStrategy, GraphCompactionStrategy, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

fn run_compaction_case(
    strategy: GraphCompactionStrategy,
    constraints: ConstraintCalculationStrategy,
    routing: EdgeRouting,
    connected_components: bool,
) {
    init_layered_options();
    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY,
        strategy,
    );
    set_node_property(
        &graph,
        LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS,
        constraints,
    );
    set_node_property(&graph, LayeredOptions::EDGE_ROUTING, routing);
    set_node_property(
        &graph,
        LayeredOptions::COMPACTION_CONNECTED_COMPONENTS,
        connected_components,
    );

    let n1 = create_node(&graph, 40.0, 30.0);
    let n2 = create_node(&graph, 40.0, 30.0);
    let n3 = create_node(&graph, 40.0, 30.0);
    let n4 = create_node(&graph, 40.0, 30.0);
    let n5 = create_node(&graph, 40.0, 30.0);

    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );

    run_layout(&graph);

    for node in [n1, n2, n3, n4, n5] {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        assert!(shape.x().is_finite());
        assert!(shape.y().is_finite());
        assert!(shape.width() > 0.0);
        assert!(shape.height() > 0.0);
    }
}

#[test]
fn compaction_none_orthogonal_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::None,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_left_orthogonal_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::Left,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_right_orthogonal_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::Right,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_left_right_constraint_locking_orthogonal_quadratic() {
    run_compaction_case(
        GraphCompactionStrategy::LeftRightConstraintLocking,
        ConstraintCalculationStrategy::Quadratic,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_left_right_connection_locking_orthogonal_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::LeftRightConnectionLocking,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_edge_length_orthogonal_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::EdgeLength,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Orthogonal,
        false,
    );
}

#[test]
fn compaction_left_splines_quadratic() {
    run_compaction_case(
        GraphCompactionStrategy::Left,
        ConstraintCalculationStrategy::Quadratic,
        EdgeRouting::Splines,
        false,
    );
}

#[test]
fn compaction_right_splines_scanline() {
    run_compaction_case(
        GraphCompactionStrategy::Right,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Splines,
        false,
    );
}

#[test]
fn compaction_polyline_left_does_not_fail() {
    run_compaction_case(
        GraphCompactionStrategy::Left,
        ConstraintCalculationStrategy::Scanline,
        EdgeRouting::Polyline,
        false,
    );
}

#[test]
fn compaction_connected_components_enabled() {
    run_compaction_case(
        GraphCompactionStrategy::LeftRightConstraintLocking,
        ConstraintCalculationStrategy::Quadratic,
        EdgeRouting::Orthogonal,
        true,
    );
}
