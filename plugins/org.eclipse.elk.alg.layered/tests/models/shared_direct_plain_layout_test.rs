use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, PortConstraints, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef, ElkNode, ElkNodeRef, ElkPortRef,
};

#[test]
fn direct_layout_test() {
    initialize_plain_java_layout();

    let graph = create_simple_graph();
    add_layered_options(&graph);

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(&graph, &mut BasicProgressMonitor::new());

    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for child in children {
        let mut child_mut = child.borrow_mut();
        let shape = child_mut.connectable().shape();
        assert!(
            shape.x() > 0.0 && shape.y() > 0.0,
            "Not all node coordinates have been set properly."
        );
    }
}

#[test]
fn plain_layout_test() {
    initialize_plain_java_layout();

    let graph = create_hierarchical_graph();
    for node in all_nodes(&graph) {
        let has_children = !node.borrow_mut().children().is_empty();
        if has_children {
            set_node_property(
                &node,
                CoreOptions::ALGORITHM,
                LayeredOptions::ALGORITHM_ID.to_string(),
            );
        }
    }

    let mut engine = RecursiveGraphLayoutEngine::new();
    engine.layout(&graph, &mut BasicProgressMonitor::new());
}

fn create_simple_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();

    let child1 = ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_label_with_text("node1", Some(ElkGraphElementRef::Node(child1.clone())));

    let child2 = ElkGraphUtil::create_node(Some(graph.clone()));
    ElkGraphUtil::create_label_with_text("node2", Some(ElkGraphElementRef::Node(child2.clone())));

    let port1 = ElkGraphUtil::create_port(Some(child1.clone()));
    let port2 = ElkGraphUtil::create_port(Some(child2.clone()));
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port1),
        ElkConnectableShapeRef::Port(port2),
    );

    graph
}

fn create_hierarchical_graph() -> ElkNodeRef {
    let parent = ElkGraphUtil::create_graph();

    let child1 = create_simple_graph();
    ElkNode::set_parent(&child1, Some(parent.clone()));
    ElkGraphUtil::create_label_with_text("child1", Some(ElkGraphElementRef::Node(child1.clone())));

    let child2 = create_simple_graph();
    ElkNode::set_parent(&child2, Some(parent.clone()));
    ElkGraphUtil::create_label_with_text("child2", Some(ElkGraphElementRef::Node(child2.clone())));

    parent
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

fn add_layered_options(graph: &ElkNodeRef) {
    set_node_property(graph, CoreOptions::DIRECTION, Direction::Right);

    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for child in children {
        set_node_dimensions(&child, 30.0, 30.0);
        set_node_property(
            &child,
            CoreOptions::NODE_SIZE_CONSTRAINTS,
            SizeConstraint::fixed(),
        );
        set_node_property(
            &child,
            CoreOptions::PORT_CONSTRAINTS,
            PortConstraints::FixedPos,
        );

        let ports: Vec<ElkPortRef> = child.borrow_mut().ports().iter().cloned().collect();
        let count = ports.len() as f64;
        let is_node1 = child
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .get(0)
            .map(|label| label.borrow().text() == "node1")
            .unwrap_or(false);

        for (idx, port) in ports.into_iter().enumerate() {
            let y = ((idx + 1) as f64) * 30.0 / (count + 1.0);
            let x = if is_node1 { 30.0 } else { 0.0 };
            port.borrow_mut().connectable().shape().set_location(x, y);
            set_port_property(
                &port,
                CoreOptions::PORT_SIDE,
                if is_node1 {
                    PortSide::East
                } else {
                    PortSide::West
                },
            );
        }
    }
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: T,
) {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: T,
) {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
