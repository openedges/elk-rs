use std::fmt;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};

use super::{index_of_arc, LGraphElement, LGraphRef, LNodeRef, LNodeWeak, LPadding, LayerRef};

pub struct LGraph {
    element: LGraphElement,
    size: KVector,
    padding: LPadding,
    offset: KVector,
    layerless_nodes: Vec<LNodeRef>,
    layers: Vec<LayerRef>,
    parent_node: Option<LNodeWeak>,
}

impl LGraph {
    pub fn new() -> LGraphRef {
        Arc::new(Mutex::new(LGraph {
            element: LGraphElement::new(),
            size: KVector::new(),
            padding: LPadding::new(),
            offset: KVector::new(),
            layerless_nodes: Vec::new(),
            layers: Vec::new(),
            parent_node: None,
        }))
    }

    pub fn graph_element(&mut self) -> &mut LGraphElement {
        &mut self.element
    }

    pub fn size(&mut self) -> &mut KVector {
        &mut self.size
    }

    pub fn size_ref(&self) -> &KVector {
        &self.size
    }

    pub fn actual_size(&self) -> KVector {
        KVector::with_values(
            self.size.x + self.padding.left + self.padding.right,
            self.size.y + self.padding.top + self.padding.bottom,
        )
    }

    pub fn padding(&mut self) -> &mut LPadding {
        &mut self.padding
    }

    pub fn padding_ref(&self) -> &LPadding {
        &self.padding
    }

    pub fn offset(&mut self) -> &mut KVector {
        &mut self.offset
    }

    pub fn offset_ref(&self) -> &KVector {
        &self.offset
    }

    pub fn layerless_nodes(&self) -> &Vec<LNodeRef> {
        &self.layerless_nodes
    }

    pub fn layerless_nodes_mut(&mut self) -> &mut Vec<LNodeRef> {
        &mut self.layerless_nodes
    }

    pub fn layers(&self) -> &Vec<LayerRef> {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut Vec<LayerRef> {
        &mut self.layers
    }

    pub fn parent_node(&self) -> Option<LNodeRef> {
        self.parent_node.as_ref().and_then(|node| node.upgrade())
    }

    pub fn set_parent_node(&mut self, parent: Option<LNodeRef>) {
        self.parent_node = parent.as_ref().map(Arc::downgrade);
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        self.element.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.element.set_property(property, value);
    }

    // --- Typed property accessors (read-only, &self) ---

    pub fn graph_properties(&self) -> EnumSet<GraphProperties> {
        self.element
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of)
    }

    pub fn direction(&self) -> Direction {
        self.element
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Undefined)
    }

    pub fn edge_routing(&self) -> EdgeRouting {
        self.element
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Undefined)
    }

    pub fn to_node_array(&self) -> Vec<Vec<LNodeRef>> {
        let mut result = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let layer_guard = layer.lock();            result.push(layer_guard.nodes().clone());
        }
        result
    }

}

impl fmt::Display for LGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let layerless = self
            .layerless_nodes
            .iter()
            .map(|node| node.lock().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let layers = self
            .layers
            .iter()
            .map(|layer| layer.lock().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        if self.layers.is_empty() {
            write!(f, "G-unlayered[{}]", layerless)
        } else if self.layerless_nodes.is_empty() {
            write!(f, "G-layered[{}]", layers)
        } else {
            write!(f, "G[layerless[{}], layers[{}]]", layerless, layers)
        }
    }
}

impl Default for LGraph {
    fn default() -> Self {
        LGraph {
            element: LGraphElement::new(),
            size: KVector::new(),
            padding: LPadding::new(),
            offset: KVector::new(),
            layerless_nodes: Vec::new(),
            layers: Vec::new(),
            parent_node: None,
        }
    }
}

impl LGraph {
    pub fn index_of_layer(&self, layer: &LayerRef) -> Option<usize> {
        index_of_arc(&self.layers, layer)
    }
}
