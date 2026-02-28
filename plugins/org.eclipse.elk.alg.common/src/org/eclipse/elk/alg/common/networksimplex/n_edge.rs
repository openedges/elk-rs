use std::any::Any;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use super::n_node::NNodeRef;

pub type NEdgeRef = Arc<Mutex<NEdge>>;

pub struct NEdge {
    pub id: i32,
    pub internal_id: usize,
    pub origin: Option<Arc<dyn Any + Send + Sync>>,
    pub source: NNodeRef,
    pub target: NNodeRef,
    pub weight: f64,
    pub delta: i32,
    pub tree_edge: bool,
}

impl NEdge {
    pub fn of() -> NEdgeBuilder {
        NEdgeBuilder::new()
    }

    pub fn of_origin(origin: Arc<dyn Any + Send + Sync>) -> NEdgeBuilder {
        NEdgeBuilder::new().origin(origin)
    }

    pub fn source(&self) -> NNodeRef {
        self.source.clone()
    }

    pub fn target(&self) -> NNodeRef {
        self.target.clone()
    }

    pub fn other(&self, some: &NNodeRef) -> NNodeRef {
        if Arc::ptr_eq(some, &self.source) {
            return self.target.clone();
        }
        if Arc::ptr_eq(some, &self.target) {
            return self.source.clone();
        }
        panic!("Node not part of edge");
    }

    pub fn reverse(edge: &NEdgeRef) {
        let (old_source, old_target) = {
            let guard = edge.lock().expect("edge lock");
            (guard.source.clone(), guard.target.clone())
        };

        {
            let mut edge_guard = edge.lock().expect("edge lock");
            edge_guard.source = old_target.clone();
            edge_guard.target = old_source.clone();
        }

        remove_edge_from(&old_source, edge, true);
        remove_edge_from(&old_target, edge, false);

        let old_target_ref = old_target.clone();
        {
            let Ok(mut source_guard) = old_target_ref.lock() else {
                return;
            };
            source_guard.outgoing_edges_mut().push(edge.clone());
        }
        let old_source_ref = old_source.clone();
        {
            let Ok(mut target_guard) = old_source_ref.lock() else {
                return;
            };
            target_guard.incoming_edges_mut().push(edge.clone());
        }
    }
}

pub struct NEdgeBuilder {
    id: i32,
    origin: Option<Arc<dyn Any + Send + Sync>>,
    weight: f64,
    delta: i32,
    source: Option<NNodeRef>,
    target: Option<NNodeRef>,
}

impl NEdgeBuilder {
    fn new() -> Self {
        NEdgeBuilder {
            id: 0,
            origin: None,
            weight: 0.0,
            delta: 1,
            source: None,
            target: None,
        }
    }

    pub fn id(mut self, id: i32) -> Self {
        self.id = id;
        self
    }

    pub fn origin(mut self, origin: Arc<dyn Any + Send + Sync>) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn delta(mut self, delta: i32) -> Self {
        self.delta = delta;
        self
    }

    pub fn source(mut self, source: NNodeRef) -> Self {
        self.source = Some(source);
        self
    }

    pub fn target(mut self, target: NNodeRef) -> Self {
        self.target = Some(target);
        self
    }

    pub fn create(self) -> NEdgeRef {
        let source = self.source.expect("source must be set");
        let target = self.target.expect("target must be set");
        if Arc::ptr_eq(&source, &target) {
            panic!("Network simplex does not support self-loops");
        }

        let edge_ref = Arc::new(Mutex::new(NEdge {
            id: self.id,
            internal_id: 0,
            origin: self.origin,
            source: source.clone(),
            target: target.clone(),
            weight: self.weight,
            delta: self.delta,
            tree_edge: false,
        }));

        if let Ok(mut source_guard) = source.lock() {
            source_guard.outgoing_edges_mut().push(edge_ref.clone());
        }
        if let Ok(mut target_guard) = target.lock() {
            target_guard.incoming_edges_mut().push(edge_ref.clone());
        }

        edge_ref
    }
}

fn remove_edge_from(node: &NNodeRef, edge: &NEdgeRef, outgoing: bool) {
    if let Ok(mut node_guard) = node.lock() {
        let list = if outgoing {
            node_guard.outgoing_edges_mut()
        } else {
            node_guard.incoming_edges_mut()
        };
        list.retain(|candidate| !Arc::ptr_eq(candidate, edge));
    }
}
