use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Maybe;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkEdgeSectionRef, ElkNodeRef,
};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

pub(super) fn parse_lenient_json(input: &str) -> Value {
    json5::from_str(input).expect("lenient json")
}

pub(super) fn to_json_graph_and_back(graph: &str) -> String {
    let shared = Rc::new(RefCell::new(parse_lenient_json(graph)));
    let mut importer = Maybe::default();
    let root = ElkGraphJson::for_graph_shared(shared.clone())
        .remember_importer(&mut importer)
        .to_elk()
        .unwrap();
    importer
        .get_mut()
        .expect("importer")
        .transfer_layout(&root)
        .unwrap();
    let serialized = serde_json::to_string(&*shared.borrow()).unwrap();
    serialized
}

pub(super) fn node_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    node.borrow_mut().children().iter().cloned().collect()
}

pub(super) fn node_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.contained_edges().iter().cloned().collect()
}

pub(super) fn node_ports(
    node: &ElkNodeRef,
) -> Vec<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.ports().iter().cloned().collect()
}

pub(super) fn node_labels(
    node: &ElkNodeRef,
) -> Vec<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

pub(super) fn node_identifier(node: &ElkNodeRef) -> Option<String> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|value| value.to_string())
}

pub(super) fn find_node(nodes: &[ElkNodeRef], id: &str) -> ElkNodeRef {
    nodes
        .iter()
        .find(|node| node_identifier(node).as_deref() == Some(id))
        .cloned()
        .expect("node")
}

pub(super) fn edge_sections(edge: &ElkEdgeRef) -> Vec<ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    edge_mut.sections().iter().cloned().collect()
}

pub(super) fn find_section(sections: &[ElkEdgeSectionRef], id: &str) -> ElkEdgeSectionRef {
    sections
        .iter()
        .find(|section| section.borrow().identifier() == Some(id))
        .cloned()
        .expect("section")
}

pub(super) fn node_has_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> bool {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
}

pub(super) fn node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

pub(super) fn edge_property<T: Clone + Send + Sync + 'static>(
    edge: &ElkEdgeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut edge_ref = edge.borrow_mut();
    edge_ref.element().properties_mut().get_property(property)
}

pub(super) fn port_property<T: Clone + Send + Sync + 'static>(
    port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef,
    property: &Property<T>,
) -> Option<T> {
    let mut port_ref = port.borrow_mut();
    port_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

pub(super) fn label_property<T: Clone + Send + Sync + 'static>(
    label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    property: &Property<T>,
) -> Option<T> {
    let mut label_ref = label.borrow_mut();
    label_ref
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

pub(super) fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

pub(super) fn set_edge_property<T: Clone + Send + Sync + 'static>(
    edge: &ElkEdgeRef,
    property: &Property<T>,
    value: T,
) {
    let mut edge_ref = edge.borrow_mut();
    edge_ref
        .element()
        .properties_mut()
        .set_property(property, Some(value));
}

pub(super) fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    let mut port_ref = port.borrow_mut();
    port_ref
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

pub(super) fn set_label_property<T: Clone + Send + Sync + 'static>(
    label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    property: &Property<T>,
    value: T,
) {
    let mut label_ref = label.borrow_mut();
    label_ref
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

pub(super) fn set_node_identifier(node: &ElkNodeRef, value: &str) {
    let mut node_ref = node.borrow_mut();
    node_ref
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(Some(value.to_string()));
}

pub(super) fn set_node_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_ref = node.borrow_mut();
    node_ref.connectable().shape().set_location(x, y);
}
