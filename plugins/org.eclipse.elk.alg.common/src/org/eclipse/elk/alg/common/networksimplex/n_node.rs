use std::any::Any;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use super::n_edge::NEdgeRef;
use super::n_graph::NGraph;

pub type NNodeRef = Arc<Mutex<NNode>>;

pub struct NNode {
    pub id: i32,
    pub internal_id: usize,
    pub origin: Option<Arc<dyn Any + Send + Sync>>,
    pub type_label: String,
    pub layer: i32,
    outgoing_edges: Vec<NEdgeRef>,
    incoming_edges: Vec<NEdgeRef>,
    pub tree_node: bool,
    pub unknown_cutvalues: Vec<NEdgeRef>,
}

impl NNode {
    pub fn of() -> NNodeBuilder {
        NNodeBuilder::new()
    }

    pub fn outgoing_edges(&self) -> &Vec<NEdgeRef> {
        &self.outgoing_edges
    }

    pub fn outgoing_edges_mut(&mut self) -> &mut Vec<NEdgeRef> {
        &mut self.outgoing_edges
    }

    pub fn incoming_edges(&self) -> &Vec<NEdgeRef> {
        &self.incoming_edges
    }

    pub fn incoming_edges_mut(&mut self) -> &mut Vec<NEdgeRef> {
        &mut self.incoming_edges
    }

    pub fn connected_edges(&self) -> Vec<NEdgeRef> {
        // Java parity: NNode#getConnectedEdges() returns incoming edges first, then outgoing edges.
        let mut edges = Vec::with_capacity(self.incoming_edges.len() + self.outgoing_edges.len());
        edges.extend(self.incoming_edges.iter().cloned());
        edges.extend(self.outgoing_edges.iter().cloned());
        edges
    }

    /// Number of connected edges without allocation.
    #[inline]
    pub fn connected_edge_count(&self) -> usize {
        self.incoming_edges.len() + self.outgoing_edges.len()
    }
}

pub struct NNodeBuilder {
    node: NNode,
}

impl NNodeBuilder {
    fn new() -> Self {
        NNodeBuilder {
            node: NNode {
                id: 0,
                internal_id: 0,
                origin: None,
                type_label: String::new(),
                layer: 0,
                outgoing_edges: Vec::new(),
                incoming_edges: Vec::new(),
                tree_node: false,
                unknown_cutvalues: Vec::new(),
            },
        }
    }

    pub fn id(mut self, id: i32) -> Self {
        self.node.id = id;
        self
    }

    pub fn origin(mut self, origin: Arc<dyn Any + Send + Sync>) -> Self {
        self.node.origin = Some(origin);
        self
    }

    pub fn type_label(mut self, type_label: impl Into<String>) -> Self {
        self.node.type_label = type_label.into();
        self
    }

    pub fn create(self, graph: &mut NGraph) -> NNodeRef {
        let node_ref = Arc::new(Mutex::new(self.node));
        graph.nodes.push(node_ref.clone());
        node_ref
    }
}
