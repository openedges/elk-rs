use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkGraphFactory, ElkLabelRef, ElkNodeRef,
    ElkPortRef,
};

pub struct ElkReflect;

type NewInstanceFn = Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>;
type CloneFn = Box<dyn Fn(&dyn Any) -> Option<Box<dyn Any + Send + Sync>> + Send + Sync>;

static NEW_REGISTRY: OnceLock<Mutex<HashMap<TypeId, NewInstanceFn>>> = OnceLock::new();
static CLONE_REGISTRY: OnceLock<Mutex<HashMap<TypeId, CloneFn>>> = OnceLock::new();

impl ElkReflect {
    pub fn register<T: Send + Sync + 'static>(
        new_instance: Option<fn() -> T>,
        clone_fn: Option<fn(&T) -> T>,
    ) {
        if let Some(new_instance) = new_instance {
            let registry = NEW_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
            registry.lock().unwrap().insert(
                TypeId::of::<T>(),
                Box::new(move || Box::new(new_instance()) as Box<dyn Any + Send + Sync>),
            );
        }
        if let Some(clone_fn) = clone_fn {
            let registry = CLONE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
            registry.lock().unwrap().insert(
                TypeId::of::<T>(),
                Box::new(move |value: &dyn Any| {
                    value
                        .downcast_ref::<T>()
                        .map(|typed| Box::new(clone_fn(typed)) as Box<dyn Any + Send + Sync>)
                }),
            );
        }
    }

    pub fn register_new_instance<T: Send + Sync + 'static>(new_instance: fn() -> T) {
        Self::register(Some(new_instance), None);
    }

    pub fn register_clone<T: Send + Sync + 'static>(clone_fn: fn(&T) -> T) {
        Self::register::<T>(None, Some(clone_fn));
    }

    pub fn register_default_clone<T: Clone + Send + Sync + 'static>() {
        Self::register_clone::<T>(|value| value.clone());
    }

    pub fn new_instance<T: Send + Sync + 'static>() -> Option<T> {
        let registry = NEW_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        registry
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|ctor| ctor().downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    pub fn clone_value<T: Send + Sync + 'static>(value: &T) -> Option<T> {
        let registry = CLONE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        registry
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|clone_fn| clone_fn(value as &dyn Any))
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    pub fn clone_any(value: &dyn Any) -> Option<Box<dyn Any + Send + Sync>> {
        let registry = CLONE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        registry
            .lock()
            .unwrap()
            .get(&value.type_id())
            .and_then(|clone_fn| clone_fn(value))
    }

    pub fn has_clone<T: Send + Sync + 'static>() -> bool {
        let registry = CLONE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        registry.lock().unwrap().contains_key(&TypeId::of::<T>())
    }
}

pub struct ElkGraphUtil;

impl ElkGraphUtil {
    pub fn create_graph() -> ElkNodeRef {
        Self::create_node(None)
    }

    pub fn create_node(parent: Option<ElkNodeRef>) -> ElkNodeRef {
        let node = ElkGraphFactory::instance().create_elk_node();
        if let Some(parent) = parent {
            crate::org::eclipse::elk::graph::ElkNode::set_parent(&node, Some(parent));
        }
        node
    }

    pub fn create_port(parent: Option<ElkNodeRef>) -> crate::org::eclipse::elk::graph::ElkPortRef {
        let port = ElkGraphFactory::instance().create_elk_port();
        if let Some(parent) = parent {
            crate::org::eclipse::elk::graph::ElkPort::set_parent(&port, Some(parent));
        }
        port
    }

    pub fn create_edge(containing_node: Option<ElkNodeRef>) -> ElkEdgeRef {
        let edge = ElkGraphFactory::instance().create_elk_edge();
        if let Some(containing_node) = containing_node {
            crate::org::eclipse::elk::graph::ElkEdge::set_containing_node(&edge, Some(containing_node));
        }
        edge
    }

    pub fn create_label(parent: Option<ElkGraphElementRef>) -> ElkLabelRef {
        let label = ElkGraphFactory::instance().create_elk_label();
        if let Some(parent) = parent {
            crate::org::eclipse::elk::graph::ElkLabel::set_parent(&label, Some(parent));
        }
        label
    }

    pub fn create_label_with_text(text: impl Into<String>, parent: Option<ElkGraphElementRef>) -> ElkLabelRef {
        let label = Self::create_label(parent);
        label.borrow_mut().set_text(text);
        label
    }

    pub fn create_simple_edge(
        source: ElkConnectableShapeRef,
        target: ElkConnectableShapeRef,
    ) -> ElkEdgeRef {
        let edge = Self::create_edge(None);
        crate::org::eclipse::elk::graph::ElkEdge::add_source(&edge, source);
        crate::org::eclipse::elk::graph::ElkEdge::add_target(&edge, target);
        Self::update_containment(&edge);
        edge
    }

    pub fn create_hyperedge(
        sources: impl IntoIterator<Item = ElkConnectableShapeRef>,
        targets: impl IntoIterator<Item = ElkConnectableShapeRef>,
    ) -> ElkEdgeRef {
        let edge = Self::create_edge(None);
        for source in sources {
            crate::org::eclipse::elk::graph::ElkEdge::add_source(&edge, source);
        }
        for target in targets {
            crate::org::eclipse::elk::graph::ElkEdge::add_target(&edge, target);
        }
        Self::update_containment(&edge);
        edge
    }

    pub fn update_containment(edge: &ElkEdgeRef) {
        let containing = Self::find_best_edge_containment(edge);
        crate::org::eclipse::elk::graph::ElkEdge::set_containing_node(edge, containing);
    }

    pub fn find_best_edge_containment(edge: &ElkEdgeRef) -> Option<ElkNodeRef> {
        let edge_borrow = edge.borrow();
        let source_len = edge_borrow.sources_ro().len();
        let target_len = edge_borrow.targets_ro().len();
        drop(edge_borrow);

        match source_len + target_len {
            0 => panic!("The edge must have at least one source or target."),
            1 => {
                let edge_borrow = edge.borrow();
                if source_len == 0 {
                    let target = edge_borrow.targets_ro().get(0);
                    drop(edge_borrow);
                    return target
                        .and_then(|shape| Self::connectable_shape_to_node(&shape))
                        .and_then(|node| node.borrow().parent());
                }
                let source = edge_borrow.sources_ro().get(0);
                drop(edge_borrow);
                return source
                    .and_then(|shape| Self::connectable_shape_to_node(&shape))
                    .and_then(|node| node.borrow().parent());
            }
            _ => {}
        }

        let edge_borrow = edge.borrow();
        if edge_borrow.sources_ro().len() == 1 && edge_borrow.targets_ro().len() == 1 {
            let source_shape = edge_borrow.sources_ro().get(0).unwrap();
            let target_shape = edge_borrow.targets_ro().get(0).unwrap();
            let source_node = Self::connectable_shape_to_node(&source_shape)?;
            let target_node = Self::connectable_shape_to_node(&target_shape)?;
            let source_parent = source_node.borrow().parent();
            let target_parent = target_node.borrow().parent();
            if match (&source_parent, &target_parent) {
                (Some(sp), Some(tp)) => std::rc::Rc::ptr_eq(sp, tp),
                (None, None) => true,
                _ => false,
            } {
                return source_parent;
            }
            if let Some(target_parent) = target_parent {
                if std::rc::Rc::ptr_eq(&source_node, &target_parent) {
                    return Some(source_node);
                }
            }
            if let Some(source_parent) = source_parent {
                if std::rc::Rc::ptr_eq(&target_node, &source_parent) {
                    return Some(target_node);
                }
            }
        }
        drop(edge_borrow);

        let incident_shapes = Self::all_incident_shapes(edge);
        let mut iter = incident_shapes.into_iter();
        let first_shape = iter.next()?;
        let mut common_ancestor = Self::connectable_shape_to_node(&first_shape)?;

        for shape in iter {
            let incident_node = Self::connectable_shape_to_node(&shape)?;
            if !std::rc::Rc::ptr_eq(&incident_node, &common_ancestor)
                && !Self::is_descendant(&incident_node, &common_ancestor)
            {
                let incident_parent = incident_node.borrow().parent();
                let common_parent = common_ancestor.borrow().parent();
                let siblings = match (incident_parent, common_parent) {
                    (Some(ip), Some(cp)) => std::rc::Rc::ptr_eq(&ip, &cp),
                    (None, None) => true,
                    _ => false,
                };
                if siblings {
                    common_ancestor = incident_node.borrow().parent()?;
                } else {
                    match Self::find_lowest_common_ancestor(&common_ancestor, &incident_node) {
                        Some(lca) => common_ancestor = lca,
                        None => return None,
                    }
                }
            }
        }

        Some(common_ancestor)
    }

    pub fn find_lowest_common_ancestor(
        node1: &ElkNodeRef,
        node2: &ElkNodeRef,
    ) -> Option<ElkNodeRef> {
        let ancestors1 = Self::ancestor_chain(node1, true);
        let ancestors2 = Self::ancestor_chain(node2, true);
        let mut iter1 = ancestors1.iter().rev();
        let mut iter2 = ancestors2.iter().rev();
        let mut common: Option<ElkNodeRef> = None;
        while let (Some(a1), Some(a2)) = (iter1.next(), iter2.next()) {
            if std::rc::Rc::ptr_eq(a1, a2) {
                common = Some(a1.clone());
            } else {
                break;
            }
        }
        common
    }

    fn ancestor_chain(node: &ElkNodeRef, include_node: bool) -> Vec<ElkNodeRef> {
        let mut ancestors = Vec::new();
        let mut current = if include_node {
            Some(node.clone())
        } else {
            node.borrow().parent()
        };
        while let Some(node) = current {
            ancestors.push(node.clone());
            current = node.borrow().parent();
        }
        ancestors
    }

    pub fn is_descendant(child: &ElkNodeRef, ancestor: &ElkNodeRef) -> bool {
        let mut current = child.borrow().parent();
        while let Some(node) = current {
            if std::rc::Rc::ptr_eq(&node, ancestor) {
                return true;
            }
            current = node.borrow().parent();
        }
        false
    }

    pub fn all_incident_shapes(edge: &ElkEdgeRef) -> Vec<ElkConnectableShapeRef> {
        let edge_borrow = edge.borrow();
        let mut shapes = Vec::with_capacity(edge_borrow.sources_ro().len() + edge_borrow.targets_ro().len());
        shapes.extend(edge_borrow.sources_ro().iter().cloned());
        shapes.extend(edge_borrow.targets_ro().iter().cloned());
        shapes
    }

    pub fn all_incoming_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
        let (mut edges, ports) = {
            let mut node_mut = node.borrow_mut();
            let edges: Vec<ElkEdgeRef> = node_mut.connectable().incoming_edges().iter().collect();
            let ports: Vec<ElkPortRef> = node_mut.ports().iter().cloned().collect();
            (edges, ports)
        };

        for port in ports {
            let mut port_mut = port.borrow_mut();
            edges.extend(port_mut.connectable().incoming_edges().iter());
        }

        edges
    }

    pub fn all_outgoing_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
        let (mut edges, ports) = {
            let mut node_mut = node.borrow_mut();
            let edges: Vec<ElkEdgeRef> = node_mut.connectable().outgoing_edges().iter().collect();
            let ports: Vec<ElkPortRef> = node_mut.ports().iter().cloned().collect();
            (edges, ports)
        };

        for port in ports {
            let mut port_mut = port.borrow_mut();
            edges.extend(port_mut.connectable().outgoing_edges().iter());
        }

        edges
    }

    pub fn all_incident_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
        let mut edges = Self::all_outgoing_edges(node);
        edges.extend(Self::all_incoming_edges(node));
        edges
    }

    pub fn all_incident_edges_for_shape(shape: &ElkConnectableShapeRef) -> Vec<ElkEdgeRef> {
        match shape {
            ElkConnectableShapeRef::Node(node) => Self::all_incident_edges(node),
            ElkConnectableShapeRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                let mut edges: Vec<ElkEdgeRef> =
                    port_mut.connectable().outgoing_edges().iter().collect();
                edges.extend(port_mut.connectable().incoming_edges().iter());
                edges
            }
        }
    }

    pub fn connectable_shape_to_node(
        connectable_shape: &ElkConnectableShapeRef,
    ) -> Option<ElkNodeRef> {
        match connectable_shape {
            ElkConnectableShapeRef::Node(node) => Some(node.clone()),
            ElkConnectableShapeRef::Port(port) => port.borrow().parent(),
        }
    }

    pub fn connectable_shape_to_port(
        connectable_shape: &ElkConnectableShapeRef,
    ) -> Option<crate::org::eclipse::elk::graph::ElkPortRef> {
        match connectable_shape {
            ElkConnectableShapeRef::Port(port) => Some(port.clone()),
            _ => None,
        }
    }

    pub fn containing_graph(element: &ElkGraphElementRef) -> Option<ElkNodeRef> {
        let mut current = element.clone();
        loop {
            match current {
                ElkGraphElementRef::Edge(edge) => return edge.borrow().containing_node(),
                ElkGraphElementRef::Node(node) => return node.borrow().parent(),
                ElkGraphElementRef::Port(port) => {
                    if let Some(parent) = port.borrow().parent() {
                        current = ElkGraphElementRef::Node(parent);
                    } else {
                        return None;
                    }
                }
                ElkGraphElementRef::Label(label) => {
                    if let Some(parent) = label.borrow().parent() {
                        current = parent;
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}
