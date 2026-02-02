use org_eclipse_elk_graph::org::eclipse::elk::graph::util::{ElkGraphUtil, GraphIdentifierGenerator};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdge, ElkEdgeSection, ElkGraphElementRef, ElkLabelRef, ElkNodeRef,
    ElkPortRef,
};

fn node_identifier(node: &ElkNodeRef) -> Option<String> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn port_identifier(port: &ElkPortRef) -> Option<String> {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn edge_identifier(edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef) -> Option<String> {
    let mut edge_mut = edge.borrow_mut();
    edge_mut.element().identifier().map(|id| id.to_string())
}

fn label_identifier(label: &ElkLabelRef) -> Option<String> {
    let mut label_mut = label.borrow_mut();
    label_mut
        .shape()
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn edge_section_identifier(section: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef) -> Option<String> {
    section.borrow().identifier().map(|id| id.to_string())
}

#[test]
fn generates_missing_identifiers() {
    let root = ElkGraphUtil::create_graph();
    let child = ElkGraphUtil::create_node(Some(root.clone()));
    let port = ElkGraphUtil::create_port(Some(child.clone()));
    let edge = ElkGraphUtil::create_edge(Some(root.clone()));

    ElkEdge::add_source(&edge, ElkConnectableShapeRef::Node(child.clone()));
    ElkEdge::add_target(&edge, ElkConnectableShapeRef::Port(port.clone()));

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(child.clone())));
    let section = ElkEdgeSection::new();
    ElkEdgeSection::set_parent(&section, Some(edge.clone()));

    let mut generator = GraphIdentifierGenerator::for_graph(root.clone());
    generator.assert_exists().execute();

    assert_eq!(node_identifier(&root).as_deref(), Some("G1"));
    assert!(node_identifier(&child).unwrap().starts_with('N'));
    assert!(port_identifier(&port).unwrap().starts_with('P'));
    assert!(edge_identifier(&edge).unwrap().starts_with('E'));
    assert!(label_identifier(&label).unwrap().starts_with('L'));
    assert!(edge_section_identifier(&section).unwrap().starts_with("ES"));
}

#[test]
fn validates_identifier_characters() {
    let root = ElkGraphUtil::create_graph();
    let child = ElkGraphUtil::create_node(Some(root.clone()));
    let section = ElkEdgeSection::new();

    {
        let mut child_mut = child.borrow_mut();
        child_mut
            .connectable()
            .shape()
            .graph_element()
            .set_identifier(Some("a b".to_string()));
    }
    section.borrow_mut().set_identifier(Some("1bad".to_string()));
    ElkEdgeSection::set_parent(&section, Some(ElkGraphUtil::create_edge(Some(root.clone()))));

    let mut generator = GraphIdentifierGenerator::for_graph(root.clone());
    generator.assert_valid().execute();

    assert_eq!(node_identifier(&child).as_deref(), Some("a_b"));
    assert_eq!(edge_section_identifier(&section).as_deref(), Some("_bad"));
}

#[test]
fn ensures_unique_identifiers() {
    let root = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(root.clone()));
    let node2 = ElkGraphUtil::create_node(Some(root.clone()));

    {
        let mut node1_mut = node1.borrow_mut();
        node1_mut
            .connectable()
            .shape()
            .graph_element()
            .set_identifier(Some("dup".to_string()));
    }
    {
        let mut node2_mut = node2.borrow_mut();
        node2_mut
            .connectable()
            .shape()
            .graph_element()
            .set_identifier(Some("dup".to_string()));
    }

    let mut generator = GraphIdentifierGenerator::for_graph(root.clone());
    generator.assert_unique().execute();

    let id1 = node_identifier(&node1).unwrap();
    let id2 = node_identifier(&node2).unwrap();

    assert_eq!(id1, "dup");
    assert!(id2.starts_with("dup_g"));
    assert_ne!(id1, id2);
}
