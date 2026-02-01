use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[test]
fn test_connectable_shape_to_node() {
    let node = ElkGraphUtil::create_node(None);
    let result = ElkGraphUtil::connectable_shape_to_node(&ElkConnectableShapeRef::Node(node.clone()))
        .expect("node");
    assert!(Rc::ptr_eq(&node, &result));

    let port = ElkGraphUtil::create_port(Some(node.clone()));
    let result = ElkGraphUtil::connectable_shape_to_node(&ElkConnectableShapeRef::Port(port))
        .expect("parent");
    assert!(Rc::ptr_eq(&node, &result));
}

#[test]
fn test_connectable_shape_to_port() {
    let node = ElkGraphUtil::create_node(None);
    let result = ElkGraphUtil::connectable_shape_to_port(&ElkConnectableShapeRef::Node(node));
    assert!(result.is_none());

    let parent = ElkGraphUtil::create_node(None);
    let port = ElkGraphUtil::create_port(Some(parent));
    let result = ElkGraphUtil::connectable_shape_to_port(&ElkConnectableShapeRef::Port(port.clone()))
        .expect("port");
    assert!(Rc::ptr_eq(&port, &result));
}

fn create_edge_without_containment(
    source: Option<ElkConnectableShapeRef>,
    target: Option<ElkConnectableShapeRef>,
) -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef {
    let edge = ElkGraphUtil::create_edge(None);
    if let Some(source) = source {
        edge.borrow_mut().sources().add(source);
    }
    if let Some(target) = target {
        edge.borrow_mut().targets().add(target);
    }
    edge
}

#[test]
fn test_find_best_edge_containment() {
    let graph1 = ElkGraphUtil::create_graph();

    let node1 = ElkGraphUtil::create_node(Some(graph1.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph1.clone()));
    let node3 = ElkGraphUtil::create_node(Some(graph1.clone()));
    let node3_1 = ElkGraphUtil::create_node(Some(node3.clone()));
    let node3_1_1 = ElkGraphUtil::create_node(Some(node3_1.clone()));
    let node4 = ElkGraphUtil::create_node(Some(graph1.clone()));
    let node4_1 = ElkGraphUtil::create_node(Some(node4.clone()));

    let graph2 = ElkGraphUtil::create_graph();
    let node_a = ElkGraphUtil::create_node(Some(graph2.clone()));

    let same_level_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node1.clone())),
        Some(ElkConnectableShapeRef::Node(node2.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&same_level_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let self_loop_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node1.clone())),
        Some(ElkConnectableShapeRef::Node(node1.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&self_loop_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let down_level_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node1.clone())),
        Some(ElkConnectableShapeRef::Node(node3_1.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&down_level_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let up_level_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3_1.clone())),
        Some(ElkConnectableShapeRef::Node(node2.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&up_level_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let to_child_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3.clone())),
        Some(ElkConnectableShapeRef::Node(node3_1.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&to_child_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&node3, &containment));

    let to_parent_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3_1.clone())),
        Some(ElkConnectableShapeRef::Node(node3.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&to_parent_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&node3, &containment));

    let to_grand_child_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3.clone())),
        Some(ElkConnectableShapeRef::Node(node3_1_1.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&to_grand_child_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&node3, &containment));

    let to_grand_parent_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3_1_1.clone())),
        Some(ElkConnectableShapeRef::Node(node3.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&to_grand_parent_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&node3, &containment));

    let cross_hierarchy_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node3_1.clone())),
        Some(ElkConnectableShapeRef::Node(node4_1.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&cross_hierarchy_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let cross_graph_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node1)),
        Some(ElkConnectableShapeRef::Node(node_a)),
    );
    assert!(ElkGraphUtil::find_best_edge_containment(&cross_graph_edge).is_none());

    let source_missing_edge = create_edge_without_containment(
        None,
        Some(ElkConnectableShapeRef::Node(node2.clone())),
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&source_missing_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));

    let target_missing_edge = create_edge_without_containment(
        Some(ElkConnectableShapeRef::Node(node2)),
        None,
    );
    let containment = ElkGraphUtil::find_best_edge_containment(&target_missing_edge)
        .expect("containment");
    assert!(Rc::ptr_eq(&graph1, &containment));
}

#[test]
#[should_panic(expected = "edge must have at least one source or target")]
fn test_find_best_edge_containment_with_unconnected_edge() {
    let edge = ElkGraphUtil::create_edge(None);
    ElkGraphUtil::find_best_edge_containment(&edge);
}
