#![allow(dead_code)]

use std::collections::VecDeque;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

pub fn init_layered_options() {
    initialize_plain_java_layout();
}

pub fn create_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(
        &graph,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    graph
}

pub fn run_layout(graph: &ElkNodeRef) {
    let mut provider = LayeredLayoutProvider::new();
    provider.layout(graph, &mut BasicProgressMonitor::new());
}

pub fn run_recursive_layout(graph: &ElkNodeRef) {
    ensure_layered_algorithm_for_hierarchy(graph);
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(graph, &mut monitor);
}

fn ensure_layered_algorithm_for_hierarchy(graph: &ElkNodeRef) {
    let mut queue = VecDeque::from([graph.clone()]);
    while let Some(node) = queue.pop_front() {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            let properties = node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            if properties.get_property(CoreOptions::ALGORITHM).is_none() {
                properties.set_property(
                    CoreOptions::ALGORITHM,
                    Some(LayeredOptions::ALGORITHM_ID.to_string()),
                );
            }
            node_mut.children().iter().cloned().collect()
        };
        queue.extend(children);
    }
}

pub fn create_node(parent: &ElkNodeRef, width: f64, height: f64) -> ElkNodeRef {
    let node = ElkGraphUtil::create_node(Some(parent.clone()));
    set_node_dimensions(&node, width, height);
    node
}

pub fn create_port(parent: &ElkNodeRef, width: f64, height: f64) -> ElkPortRef {
    let port = ElkGraphUtil::create_port(Some(parent.clone()));
    set_port_dimensions(&port, width, height);
    port
}

pub fn create_edge(source: ElkConnectableShapeRef, target: ElkConnectableShapeRef) -> ElkEdgeRef {
    ElkGraphUtil::create_simple_edge(source, target)
}

pub fn create_node_label(node: &ElkNodeRef, text: &str, width: f64, height: f64) -> ElkLabelRef {
    let label = ElkGraphUtil::create_label_with_text(
        text,
        Some(
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef::Node(node.clone()),
        ),
    );
    set_label_dimensions(&label, width, height);
    label
}

pub fn create_edge_label(edge: &ElkEdgeRef, text: &str, width: f64, height: f64) -> ElkLabelRef {
    let label = ElkGraphUtil::create_label_with_text(
        text,
        Some(
            org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef::Edge(edge.clone()),
        ),
    );
    set_label_dimensions(&label, width, height);
    label
}

pub fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

pub fn set_port_dimensions(port: &ElkPortRef, width: f64, height: f64) {
    let mut port_mut = port.borrow_mut();
    port_mut.connectable().shape().set_dimensions(width, height);
}

pub fn set_label_dimensions(label: &ElkLabelRef, width: f64, height: f64) {
    let mut label_mut = label.borrow_mut();
    label_mut.shape().set_dimensions(width, height);
}

pub fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

pub fn label_bounds(label: &ElkLabelRef) -> (f64, f64, f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

pub fn set_node_property<T: Clone + Send + Sync + 'static>(
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

pub fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
