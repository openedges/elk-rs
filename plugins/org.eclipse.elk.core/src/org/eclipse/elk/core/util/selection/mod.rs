use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkPortRef,
};

#[derive(Clone)]
pub struct SelectionIterator {
    edge: ElkEdgeRef,
    visited: Option<Rc<RefCell<HashSet<usize>>>>,
}

impl SelectionIterator {
    pub fn new(edge: ElkEdgeRef) -> Self {
        SelectionIterator {
            edge,
            visited: None,
        }
    }

    pub fn edge(&self) -> &ElkEdgeRef {
        &self.edge
    }

    pub fn attach_visited_set(&mut self, visited_set: Rc<RefCell<HashSet<usize>>>) {
        self.visited = Some(visited_set);
    }

    fn visited_set(&mut self) -> Rc<RefCell<HashSet<usize>>> {
        if let Some(set) = &self.visited {
            return set.clone();
        }
        let set = Rc::new(RefCell::new(HashSet::new()));
        self.visited = Some(set.clone());
        set
    }
}

pub struct DefaultSelectionIterator {
    base: SelectionIterator,
    add_ports: bool,
    follow_edge_direction: bool,
    stack: Vec<ElkGraphElementRef>,
    initialized: bool,
}

impl DefaultSelectionIterator {
    pub fn new(edge: ElkEdgeRef, add_ports: bool, follow_edge_direction: bool) -> Self {
        DefaultSelectionIterator {
            base: SelectionIterator::new(edge),
            add_ports,
            follow_edge_direction,
            stack: Vec::new(),
            initialized: false,
        }
    }

    pub fn attach_visited_set(&mut self, visited_set: Rc<RefCell<HashSet<usize>>>) {
        self.base.attach_visited_set(visited_set);
    }

    fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        let root = ElkGraphElementRef::Edge(self.base.edge().clone());
        let children = self.children_for(&root);
        self.push_children(children);
        self.initialized = true;
    }

    fn push_children(&mut self, children: Vec<ElkGraphElementRef>) {
        for child in children.into_iter().rev() {
            self.stack.push(child);
        }
    }

    fn children_for(&mut self, element: &ElkGraphElementRef) -> Vec<ElkGraphElementRef> {
        match element {
            ElkGraphElementRef::Edge(edge) => self.children_for_edge(edge),
            _ => Vec::new(),
        }
    }

    fn children_for_edge(&mut self, edge: &ElkEdgeRef) -> Vec<ElkGraphElementRef> {
        let connected_shape = {
            let edge_ref = edge.borrow();
            if self.follow_edge_direction {
                edge_ref.targets_ro().get(0)
            } else {
                edge_ref.sources_ro().get(0)
            }
        };

        let port = match connected_shape {
            Some(ElkConnectableShapeRef::Port(port)) => port,
            _ => return Vec::new(),
        };

        let port_key = port_key(&port);
        let visited = self.base.visited_set();
        {
            let mut visited_set = visited.borrow_mut();
            if visited_set.contains(&port_key) {
                return Vec::new();
            }
            visited_set.insert(port_key);
        }

        let edges: Vec<ElkEdgeRef> = {
            let mut port_mut = port.borrow_mut();
            if self.follow_edge_direction {
                port_mut.connectable().outgoing_edges().iter().collect()
            } else {
                port_mut.connectable().incoming_edges().iter().collect()
            }
        };

        let mut children = Vec::new();
        if self.add_ports {
            children.push(ElkGraphElementRef::Port(port.clone()));
        }
        children.extend(edges.into_iter().map(ElkGraphElementRef::Edge));
        children
    }
}

impl Iterator for DefaultSelectionIterator {
    type Item = ElkGraphElementRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.initialize();
        let next = self.stack.pop()?;
        let children = self.children_for(&next);
        self.push_children(children);
        Some(next)
    }
}

fn port_key(port: &ElkPortRef) -> usize {
    Rc::as_ptr(port) as usize
}
