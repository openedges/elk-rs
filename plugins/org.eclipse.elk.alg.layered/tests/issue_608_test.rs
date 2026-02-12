mod issue_support;

use issue_support::{
    create_edge, create_graph, create_node, init_layered_options, node_bounds, run_layout,
    set_node_property,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

#[test]
fn issue_608_unnecessary_crossing_minimization_preserves_order() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(&graph, CoreOptions::RANDOM_SEED, 1);

    let n1 = create_node(&graph, 0.0, 0.0);
    let n2 = create_node(&graph, 0.0, 0.0);
    let n3 = create_node(&graph, 0.0, 0.0);

    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );

    run_layout(&graph);

    let (_, y2, _, _) = node_bounds(&n2);
    let (_, y3, _, _) = node_bounds(&n3);

    assert!(
        y2 < y3,
        "expected input order to be preserved (n2 above n3), got y2={y2} y3={y3}"
    );
}
