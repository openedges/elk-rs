use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::util::selection::DefaultSelectionIterator;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef, ElkPortRef,
};

fn element_key(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
        ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
        ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
        ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
    }
}

fn port_key(port: &ElkPortRef) -> usize {
    Rc::as_ptr(port) as usize
}

#[test]
fn default_selection_iterator_with_ports_follows_direction() {
    let n1 = ElkGraphUtil::create_node(None);
    let n2 = ElkGraphUtil::create_node(None);
    let n3 = ElkGraphUtil::create_node(None);

    let p1 = ElkGraphUtil::create_port(Some(n1));
    let p2 = ElkGraphUtil::create_port(Some(n2));
    let p3 = ElkGraphUtil::create_port(Some(n3));

    let e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p1.clone()),
        ElkConnectableShapeRef::Port(p2.clone()),
    );
    let e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p2.clone()),
        ElkConnectableShapeRef::Port(p3.clone()),
    );

    let iter = DefaultSelectionIterator::new(e1, true, true);
    let elements: Vec<_> = iter.collect();
    let keys: Vec<_> = elements.iter().map(element_key).collect();

    assert_eq!(
        keys,
        vec![port_key(&p2), Rc::as_ptr(&e2) as usize, port_key(&p3)]
    );
}

#[test]
fn default_selection_iterator_without_ports_returns_edges() {
    let n1 = ElkGraphUtil::create_node(None);
    let n2 = ElkGraphUtil::create_node(None);
    let n3 = ElkGraphUtil::create_node(None);

    let p1 = ElkGraphUtil::create_port(Some(n1));
    let p2 = ElkGraphUtil::create_port(Some(n2));
    let p3 = ElkGraphUtil::create_port(Some(n3));

    let e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p1.clone()),
        ElkConnectableShapeRef::Port(p2.clone()),
    );
    let e2 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p2),
        ElkConnectableShapeRef::Port(p3),
    );

    let iter = DefaultSelectionIterator::new(e1, false, true);
    let elements: Vec<_> = iter.collect();
    assert_eq!(elements.len(), 1);

    let only = element_key(&elements[0]);
    assert_eq!(only, Rc::as_ptr(&e2) as usize);
}

#[test]
fn default_selection_iterator_respects_visited_set() {
    let n1 = ElkGraphUtil::create_node(None);
    let n2 = ElkGraphUtil::create_node(None);

    let p1 = ElkGraphUtil::create_port(Some(n1));
    let p2 = ElkGraphUtil::create_port(Some(n2));

    let e1 = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p1),
        ElkConnectableShapeRef::Port(p2.clone()),
    );

    let visited = Rc::new(RefCell::new(HashSet::new()));
    visited.borrow_mut().insert(port_key(&p2));

    let mut iter = DefaultSelectionIterator::new(e1, true, true);
    iter.attach_visited_set(visited);

    let elements: Vec<_> = iter.collect();
    assert!(elements.is_empty());
}
