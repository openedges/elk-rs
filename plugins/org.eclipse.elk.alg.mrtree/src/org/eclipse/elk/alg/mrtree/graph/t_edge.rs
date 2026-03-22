use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::{TGraphElement, TLabelRef, TNodeRef};

pub type TEdgeRef = Arc<Mutex<TEdge>>;

pub struct TEdge {
    element: TGraphElement,
    source: Option<Weak<Mutex<super::TNode>>>,
    target: Option<Weak<Mutex<super::TNode>>>,
    labels: Vec<TLabelRef>,
    bend_points: KVectorChain,
}

impl TEdge {
    pub fn new(source: &TNodeRef, target: &TNodeRef) -> TEdgeRef {
        let edge = Arc::new(Mutex::new(TEdge {
            element: TGraphElement::default(),
            source: None,
            target: None,
            labels: Vec::new(),
            bend_points: KVectorChain::new(),
        }));
        // Set target before source so add_outgoing can extract the target
        // for the direct_children cache.
        TEdge::set_target(&edge, Some(target.clone()));
        TEdge::set_source(&edge, Some(source.clone()));
        edge
    }

    pub fn element(&self) -> &TGraphElement {
        &self.element
    }

    pub fn element_mut(&mut self) -> &mut TGraphElement {
        &mut self.element
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.element_mut().get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.element.set_property(property, value);
    }

    pub fn source(&self) -> Option<TNodeRef> {
        self.source.as_ref().and_then(|node| node.upgrade())
    }

    pub fn target(&self) -> Option<TNodeRef> {
        self.target.as_ref().and_then(|node| node.upgrade())
    }

    pub fn set_source(edge: &TEdgeRef, source: Option<TNodeRef>) {
        let current = edge.lock().source();
        if let (Some(current), Some(source)) = (&current, &source) {
            if Arc::ptr_eq(current, source) {
                return;
            }
        }

        if let Some(current) = current {
            {
                let mut node_guard = current.lock();
                node_guard.remove_outgoing(edge);
            }
        }

        if let Some(source) = &source {
            {
                let mut node_guard = source.lock();
                node_guard.add_outgoing(edge.clone());
            }
        }

        {
            let mut edge_guard = edge.lock();
            edge_guard.source = source.map(|node| Arc::downgrade(&node));
        }
    }

    pub fn set_target(edge: &TEdgeRef, target: Option<TNodeRef>) {
        let current = edge.lock().target();
        if let (Some(current), Some(target)) = (&current, &target) {
            if Arc::ptr_eq(current, target) {
                return;
            }
        }

        if let Some(current) = current {
            {
                let mut node_guard = current.lock();
                node_guard.remove_incoming(edge);
            }
        }

        if let Some(target) = &target {
            {
                let mut node_guard = target.lock();
                node_guard.add_incoming(edge.clone());
            }
        }

        {
            let mut edge_guard = edge.lock();
            edge_guard.target = target.map(|node| Arc::downgrade(&node));
        }
    }

    pub fn labels(&self) -> &Vec<TLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<TLabelRef> {
        &mut self.labels
    }

    pub fn bend_points(&mut self) -> &mut KVectorChain {
        &mut self.bend_points
    }

    pub fn bend_points_ref(&self) -> &KVectorChain {
        &self.bend_points
    }
}

impl fmt::Display for TEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(source), Some(target)) = (self.source(), self.target()) {
            let source_label = source.lock_ok().map(|node| node.to_string());
            let target_label = target.lock_ok().map(|node| node.to_string());
            if let (Some(source_label), Some(target_label)) = (source_label, target_label) {
                return write!(f, "{}->{}", source_label, target_label);
            }
            write!(f, "e_{:p}", self)
        } else {
            write!(f, "e_{:p}", self)
        }
    }
}
