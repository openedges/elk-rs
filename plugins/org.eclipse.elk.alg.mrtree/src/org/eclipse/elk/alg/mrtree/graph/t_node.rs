use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::mrtree::options::internal_properties::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use super::{TEdgeRef, TGraphElement, TGraphRef, TShape};

pub type TNodeRef = Arc<Mutex<TNode>>;

pub struct TNode {
    shape: TShape,
    graph: Option<Weak<Mutex<super::TGraph>>>,
    label: Option<String>,
    outgoing_edges: Vec<TEdgeRef>,
    incoming_edges: Vec<TEdgeRef>,
    /// Pre-computed children list maintained alongside outgoing_edges.
    /// Avoids locking each edge in children() calls.
    direct_children: Vec<TNodeRef>,
}

impl TNode {
    pub fn new(id: i32, graph: Option<TGraphRef>) -> TNodeRef {
        let node = Arc::new(Mutex::new(TNode {
            shape: TShape::new(id),
            graph: graph.as_ref().map(Arc::downgrade),
            label: None,
            outgoing_edges: Vec::new(),
            incoming_edges: Vec::new(),
            direct_children: Vec::new(),
        }));

        if let Some(graph) = graph {
            {
                let mut graph_guard = graph.lock();
                graph_guard.nodes_mut().push(node.clone());
            }
        }

        node
    }

    pub fn new_with_label(id: i32, graph: Option<TGraphRef>, label: impl Into<String>) -> TNodeRef {
        let node = Self::new(id, graph);
        {
            let mut node_guard = node.lock();
            node_guard.label = Some(label.into());
        }
        node
    }

    pub fn shape(&mut self) -> &mut TShape {
        &mut self.shape
    }

    pub fn shape_ref(&self) -> &TShape {
        &self.shape
    }

    pub fn element(&self) -> &TGraphElement {
        self.shape.element()
    }

    pub fn element_mut(&mut self) -> &mut TGraphElement {
        self.shape.element_mut()
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        self.shape.element().properties()
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        self.shape.element_mut().properties_mut()
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        self.element().get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.shape.element_mut().set_property(property, value);
    }

    pub fn position(&mut self) -> &mut KVector {
        self.shape.position()
    }

    pub fn position_ref(&self) -> &KVector {
        self.shape.position_ref()
    }

    pub fn size(&mut self) -> &mut KVector {
        self.shape.size()
    }

    pub fn size_ref(&self) -> &KVector {
        self.shape.size_ref()
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }

    pub fn outgoing_edges(&self) -> &Vec<TEdgeRef> {
        &self.outgoing_edges
    }

    pub fn outgoing_edges_mut(&mut self) -> &mut Vec<TEdgeRef> {
        &mut self.outgoing_edges
    }

    pub fn incoming_edges(&self) -> &Vec<TEdgeRef> {
        &self.incoming_edges
    }

    pub fn incoming_edges_mut(&mut self) -> &mut Vec<TEdgeRef> {
        &mut self.incoming_edges
    }

    pub fn add_outgoing(&mut self, edge: TEdgeRef) {
        if self
            .outgoing_edges
            .iter()
            .any(|candidate| Arc::ptr_eq(candidate, &edge))
        {
            return;
        }
        // Pre-extract target to avoid edge locks in children()
        {
            let guard = edge.lock();
            if let Some(target) = guard.target() {
                self.direct_children.push(target);
            }
        }
        self.outgoing_edges.push(edge);
    }

    pub fn add_incoming(&mut self, edge: TEdgeRef) {
        if self
            .incoming_edges
            .iter()
            .any(|candidate| Arc::ptr_eq(candidate, &edge))
        {
            return;
        }
        self.incoming_edges.push(edge);
    }

    pub fn remove_outgoing(&mut self, edge: &TEdgeRef) {
        if let Some(index) = self
            .outgoing_edges
            .iter()
            .position(|candidate| Arc::ptr_eq(candidate, edge))
        {
            self.outgoing_edges.remove(index);
            // Keep direct_children in sync
            {
                let guard = edge.lock();
                if let Some(target) = guard.target() {
                    if let Some(ci) = self
                        .direct_children
                        .iter()
                        .position(|c| Arc::ptr_eq(c, &target))
                    {
                        self.direct_children.remove(ci);
                    }
                }
            }
        }
    }

    pub fn remove_incoming(&mut self, edge: &TEdgeRef) {
        if let Some(index) = self
            .incoming_edges
            .iter()
            .position(|candidate| Arc::ptr_eq(candidate, edge))
        {
            self.incoming_edges.remove(index);
        }
    }

    /// Replace outgoing edges and rebuild direct_children cache.
    /// Use instead of outgoing_edges_mut().clear() + extend().
    pub fn replace_outgoing_edges(&mut self, edges: Vec<TEdgeRef>) {
        self.direct_children.clear();
        for edge in &edges {
            {
                let guard = edge.lock();
                if let Some(target) = guard.target() {
                    self.direct_children.push(target);
                }
            }
        }
        self.outgoing_edges = edges;
    }

    pub fn parent(&self) -> Option<TNodeRef> {
        self.incoming_edges
            .first()
            .and_then(|edge| edge.lock().source())
    }

    pub fn children(&self) -> Vec<TNodeRef> {
        self.direct_children.clone()
    }

    pub fn children_copy(&self) -> Vec<TNodeRef> {
        self.direct_children.clone()
    }

    pub fn add_child(node: &TNodeRef, child: &TNodeRef) {
        let edge = super::TEdge::new(node, child);
        {
            let mut edge_guard = edge.lock();
            edge_guard
                .element_mut()
                .set_property(InternalProperties::DUMMY, Some(true));
        }
        {
            let node_guard = node.lock();
            if let Some(graph) = node_guard.graph.as_ref().and_then(|graph| graph.upgrade()) {
                {
                    let mut graph_guard = graph.lock();
                    graph_guard.edges_mut().push(edge);
                }
            }
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.outgoing_edges.is_empty()
    }

    pub fn id(&self) -> i32 {
        self.shape.element().id
    }

    pub fn set_id(&mut self, id: i32) {
        self.shape.element_mut().id = id;
    }
}

impl fmt::Display for TNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(label) = self.label.as_ref() {
            if label.is_empty() {
                write!(f, "n_{}", self.shape.element().id)
            } else {
                write!(f, "n_{}", label)
            }
        } else {
            write!(f, "n_{}", self.shape.element().id)
        }
    }
}
