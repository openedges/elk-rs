use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use super::{TEdgeRef, TNodeRef};

pub type TGraphRef = Arc<Mutex<TGraph>>;

pub struct TGraph {
    properties: MapPropertyHolder,
    nodes: Vec<TNodeRef>,
    edges: Vec<TEdgeRef>,
}

impl TGraph {
    pub fn new() -> TGraphRef {
        Arc::new(Mutex::new(TGraph {
            properties: MapPropertyHolder::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }))
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        self.properties.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.properties.set_property(property, value);
    }

    pub fn copy_properties(&mut self, other: &MapPropertyHolder) {
        self.properties.copy_properties(other);
    }

    pub fn nodes(&self) -> &Vec<TNodeRef> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<TNodeRef> {
        &mut self.nodes
    }

    pub fn edges(&self) -> &Vec<TEdgeRef> {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut Vec<TEdgeRef> {
        &mut self.edges
    }
}

impl Default for TGraph {
    fn default() -> Self {
        TGraph {
            properties: MapPropertyHolder::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}
