mod issue_support;

use issue_support::{
    create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

#[test]
fn test_partition_order() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(&graph, CoreOptions::PARTITIONING_ACTIVATE, true);

    let p0_a = create_node(&graph, 30.0, 30.0);
    let p0_b = create_node(&graph, 30.0, 30.0);
    let p1_a = create_node(&graph, 30.0, 30.0);
    let p1_b = create_node(&graph, 30.0, 30.0);

    set_node_property(&p0_a, CoreOptions::PARTITIONING_PARTITION, 0);
    set_node_property(&p0_b, CoreOptions::PARTITIONING_PARTITION, 0);
    set_node_property(&p1_a, CoreOptions::PARTITIONING_PARTITION, 1);
    set_node_property(&p1_b, CoreOptions::PARTITIONING_PARTITION, 1);

    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(p0_a.clone()),
        ElkConnectableShapeRef::Node(p1_a.clone()),
    );
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(p0_b.clone()),
        ElkConnectableShapeRef::Node(p1_b.clone()),
    );
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(p0_a.clone()),
        ElkConnectableShapeRef::Node(p1_b.clone()),
    );

    run_layout(&graph);

    let p0_nodes = [p0_a, p0_b];
    let p1_nodes = [p1_a, p1_b];

    let p0_rightmost = p0_nodes
        .iter()
        .map(|node| {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.x() + shape.width()
        })
        .fold(f64::NEG_INFINITY, f64::max);
    let p1_leftmost = p1_nodes
        .iter()
        .map(|node| node.borrow_mut().connectable().shape().x())
        .fold(f64::INFINITY, f64::min);

    assert!(
        p0_rightmost < p1_leftmost,
        "partition order must hold: rightmost(p0)={} leftmost(p1)={}",
        p0_rightmost,
        p1_leftmost
    );
}

#[test]
fn test_partition_order_non_consecutive() {
    init_layered_options();

    let graph = create_graph();
    set_node_property(&graph, CoreOptions::PARTITIONING_ACTIVATE, true);

    let p1 = create_node(&graph, 30.0, 30.0);
    let p3 = create_node(&graph, 30.0, 30.0);

    set_node_property(&p1, CoreOptions::PARTITIONING_PARTITION, 1);
    set_node_property(&p3, CoreOptions::PARTITIONING_PARTITION, 3);

    // Create an edge that violates partition order to exercise reversal logic.
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(p3.clone()),
        ElkConnectableShapeRef::Node(p1.clone()),
    );

    run_layout(&graph);

    let p1_rightmost = {
        let mut node_mut = p1.borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.x() + shape.width()
    };
    let p3_leftmost = {
        let mut node_mut = p3.borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.x()
    };

    assert!(
        p1_rightmost < p3_leftmost,
        "partition order must hold for non-consecutive partitions: rightmost(p1)={} leftmost(p3)={}",
        p1_rightmost,
        p3_leftmost
    );
}
