use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkNodeRef, ElkPortRef,
};

#[test]
fn spline_edge_routing_no_panic() {
    LayoutMetaDataService::get_instance();

    let root = ElkGraphUtil::create_graph();
    set_node_property(
        &root,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(&root, CoreOptions::EDGE_ROUTING, EdgeRouting::Splines);

    let node_a = ElkGraphUtil::create_node(Some(root.clone()));
    let node_b = ElkGraphUtil::create_node(Some(root.clone()));
    let node_c = ElkGraphUtil::create_node(Some(root.clone()));

    set_dimensions(&node_a, 30.0, 30.0);
    set_dimensions(&node_b, 30.0, 30.0);
    set_dimensions(&node_c, 30.0, 30.0);

    let port_a = ElkGraphUtil::create_port(Some(node_a.clone()));
    set_port_side(&port_a, PortSide::East);
    set_port_dimensions(&port_a, 5.0, 5.0);

    let port_b = ElkGraphUtil::create_port(Some(node_b.clone()));
    set_port_side(&port_b, PortSide::West);
    set_port_dimensions(&port_b, 5.0, 5.0);

    let port_c = ElkGraphUtil::create_port(Some(node_c.clone()));
    set_port_side(&port_c, PortSide::West);
    set_port_dimensions(&port_c, 5.0, 5.0);

    let _edge1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port_a.clone()),
        ElkConnectableShapeRef::Port(port_b.clone()),
    );
    let _edge2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port_a.clone()),
        ElkConnectableShapeRef::Port(port_c.clone()),
    );

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(&root, &mut monitor);
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_port_dimensions(port: &ElkPortRef, width: f64, height: f64) {
    let mut port_mut = port.borrow_mut();
    port_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_side(port: &ElkPortRef, side: PortSide) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(side));
}
