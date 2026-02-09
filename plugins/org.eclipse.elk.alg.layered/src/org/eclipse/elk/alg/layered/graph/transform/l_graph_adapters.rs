use std::any::Any;
use std::cell::Cell;
use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, LabelSide, PortConstraints, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    EdgeAdapter, GraphAdapter, GraphElementAdapter, LabelAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LLabelRef, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

/// Factory for creating LGraph adapters, analogous to Java's LGraphAdapters.
pub struct LGraphAdapters;

impl LGraphAdapters {
    /// Adapt an LGraph for use with NodeDimensionCalculation.
    /// Only Normal nodes are included (matching Java behavior).
    pub fn adapt(graph: &mut LGraph) -> LGraphAdapter {
        LGraphAdapter::new(graph)
    }
}

/// Adapter wrapping an LGraph as a GraphAdapter.
pub struct LGraphAdapter {
    nodes: Vec<LNodeAdapter>,
    volatile_id: Cell<i32>,
    // Store graph-level properties we need
    layout_direction: org_eclipse_elk_core::org::eclipse::elk::core::options::Direction,
}

impl LGraphAdapter {
    fn new(graph: &mut LGraph) -> Self {
        let layout_direction = graph
            .graph_element()
            .get_property(CoreOptions::DIRECTION)
            .unwrap_or(org_eclipse_elk_core::org::eclipse::elk::core::options::Direction::Undefined);

        // Collect all Normal nodes from layers (matching Java LGraphAdapters.adapt behavior)
        let mut node_refs: Vec<LNodeRef> = Vec::new();
        for layer in graph.layers().clone() {
            if let Ok(layer_guard) = layer.lock() {
                for node in layer_guard.nodes() {
                    if let Ok(node_guard) = node.lock() {
                        if node_guard.node_type() == NodeType::Normal {
                            drop(node_guard);
                            node_refs.push(node.clone());
                        }
                    }
                }
            }
        }

        let nodes = node_refs
            .into_iter()
            .map(|n| LNodeAdapter::new(n))
            .collect();

        LGraphAdapter {
            nodes,
            volatile_id: Cell::new(0),
            layout_direction,
        }
    }

    pub fn layout_direction(
        &self,
    ) -> org_eclipse_elk_core::org::eclipse::elk::core::options::Direction {
        self.layout_direction
    }
}

impl GraphElementAdapter<LNodeRef> for LGraphAdapter {
    fn get_size(&self) -> KVector {
        KVector::new()
    }

    fn set_size(&self, _size: KVector) {}

    fn get_position(&self) -> KVector {
        KVector::new()
    }

    fn set_position(&self, _pos: KVector) {}

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        // Direction is the only graph-level property needed by NodeDimensionCalculation
        if prop.id() == CoreOptions::DIRECTION.id() {
            let boxed: Box<dyn Any> = Box::new(self.layout_direction);
            if let Ok(casted) = boxed.downcast::<P>() {
                return Some(*casted);
            }
        }
        None
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, _prop: &Property<P>) -> bool {
        false
    }

    fn get_volatile_id(&self) -> i32 {
        self.volatile_id.get()
    }

    fn set_volatile_id(&self, id: i32) {
        self.volatile_id.set(id);
    }
}

impl GraphAdapter<LNodeRef> for LGraphAdapter {
    type Node = LNodeRef;
    type NodeAdapter = LNodeAdapter;

    fn get_nodes(&self) -> Vec<Self::NodeAdapter> {
        self.nodes.clone()
    }
}

/// Adapter wrapping an LNode as a NodeAdapter.
#[derive(Clone)]
pub struct LNodeAdapter {
    node: LNodeRef,
    volatile_id: Cell<i32>,
}

impl LNodeAdapter {
    fn new(node: LNodeRef) -> Self {
        LNodeAdapter {
            node,
            volatile_id: Cell::new(0),
        }
    }
}

impl GraphElementAdapter<LNodeRef> for LNodeAdapter {
    fn get_size(&self) -> KVector {
        if let Ok(mut node) = self.node.lock() {
            *node.shape().size_ref()
        } else {
            KVector::new()
        }
    }

    fn set_size(&self, size: KVector) {
        if let Ok(mut node) = self.node.lock() {
            let s = node.shape().size();
            s.x = size.x;
            s.y = size.y;
        }
    }

    fn get_position(&self) -> KVector {
        if let Ok(mut node) = self.node.lock() {
            *node.shape().position_ref()
        } else {
            KVector::new()
        }
    }

    fn set_position(&self, pos: KVector) {
        if let Ok(mut node) = self.node.lock() {
            let p = node.shape().position();
            p.x = pos.x;
            p.y = pos.y;
        }
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        if let Ok(mut node) = self.node.lock() {
            node.get_property(prop)
        } else {
            None
        }
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        if let Ok(mut node) = self.node.lock() {
            node.shape().graph_element().properties().has_property(prop)
        } else {
            false
        }
    }

    fn get_volatile_id(&self) -> i32 {
        self.volatile_id.get()
    }

    fn set_volatile_id(&self, id: i32) {
        self.volatile_id.set(id);
    }
}

impl NodeAdapter<LNodeRef> for LNodeAdapter {
    type Graph = LGraphAdapter;
    type Label = LLabelRef;
    type LabelAdapter = LLabelAdapter;
    type Port = LPortRef;
    type PortAdapter = LPortAdapter;
    type Edge = LEdgeRef;
    type EdgeAdapter = LEdgeAdapter;

    fn get_graph(&self) -> Option<Self::Graph> {
        None
    }

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        if let Ok(node) = self.node.lock() {
            node.labels()
                .iter()
                .map(|l| LLabelAdapter::new(l.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_ports(&self) -> Vec<Self::PortAdapter> {
        if let Ok(node) = self.node.lock() {
            node.ports()
                .iter()
                .map(|p| LPortAdapter::new(p.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter> {
        let ports = if let Ok(node) = self.node.lock() {
            node.ports().clone()
        } else {
            return Vec::new();
        };
        let mut edges = Vec::new();
        for port in ports {
            if let Ok(port_guard) = port.lock() {
                for edge in port_guard.incoming_edges() {
                    edges.push(LEdgeAdapter::new(edge.clone()));
                }
            }
        }
        edges
    }

    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter> {
        let ports = if let Ok(node) = self.node.lock() {
            node.ports().clone()
        } else {
            return Vec::new();
        };
        let mut edges = Vec::new();
        for port in ports {
            if let Ok(port_guard) = port.lock() {
                for edge in port_guard.outgoing_edges() {
                    edges.push(LEdgeAdapter::new(edge.clone()));
                }
            }
        }
        edges
    }

    fn sort_port_list(&self) {
        if let Ok(mut node) = self.node.lock() {
            let constraints = node
                .get_property(CoreOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined);
            if constraints.is_order_fixed() {
                node.ports_mut().sort_by(|a, b| {
                    let side_a = a
                        .lock()
                        .map(|p| p.side())
                        .unwrap_or(PortSide::Undefined);
                    let side_b = b
                        .lock()
                        .map(|p| p.side())
                        .unwrap_or(PortSide::Undefined);
                    let side_cmp = (side_a as i32).cmp(&(side_b as i32));
                    if side_cmp != Ordering::Equal {
                        return side_cmp;
                    }
                    let idx_a = a
                        .lock()
                        .ok()
                        .and_then(|mut p| p.get_property(CoreOptions::PORT_INDEX));
                    let idx_b = b
                        .lock()
                        .ok()
                        .and_then(|mut p| p.get_property(CoreOptions::PORT_INDEX));
                    if let (Some(ia), Some(ib)) = (idx_a, idx_b) {
                        let idx_cmp = ia.cmp(&ib);
                        if idx_cmp != Ordering::Equal {
                            return idx_cmp;
                        }
                    }
                    // Position tiebreak
                    match side_a {
                        PortSide::North => {
                            let x_a = a.lock().map(|mut p| p.shape().position_ref().x).unwrap_or(0.0);
                            let x_b = b.lock().map(|mut p| p.shape().position_ref().x).unwrap_or(0.0);
                            x_a.partial_cmp(&x_b).unwrap_or(Ordering::Equal)
                        }
                        PortSide::East => {
                            let y_a = a.lock().map(|mut p| p.shape().position_ref().y).unwrap_or(0.0);
                            let y_b = b.lock().map(|mut p| p.shape().position_ref().y).unwrap_or(0.0);
                            y_a.partial_cmp(&y_b).unwrap_or(Ordering::Equal)
                        }
                        PortSide::South => {
                            let x_a = a.lock().map(|mut p| p.shape().position_ref().x).unwrap_or(0.0);
                            let x_b = b.lock().map(|mut p| p.shape().position_ref().x).unwrap_or(0.0);
                            x_b.partial_cmp(&x_a).unwrap_or(Ordering::Equal)
                        }
                        PortSide::West => {
                            let y_a = a.lock().map(|mut p| p.shape().position_ref().y).unwrap_or(0.0);
                            let y_b = b.lock().map(|mut p| p.shape().position_ref().y).unwrap_or(0.0);
                            y_b.partial_cmp(&y_a).unwrap_or(Ordering::Equal)
                        }
                        _ => Ordering::Equal,
                    }
                });
            }
        }
    }

    fn sort_port_list_by<F>(&self, mut comparator: F)
    where
        F: FnMut(&Self::Port, &Self::Port) -> Ordering,
    {
        if let Ok(mut node) = self.node.lock() {
            let constraints = node
                .get_property(CoreOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined);
            if constraints.is_order_fixed() {
                node.ports_mut().sort_by(|a, b| comparator(a, b));
            }
        }
    }

    fn is_compound_node(&self) -> bool {
        if let Ok(mut node) = self.node.lock() {
            // Check COMPOUND_NODE property (set during import for nodes with nested graphs)
            let is_compound = node
                .get_property(InternalProperties::COMPOUND_NODE)
                .unwrap_or(false);
            let inside_self_loops = node
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false);
            is_compound || inside_self_loops
        } else {
            false
        }
    }

    fn get_padding(&self) -> ElkPadding {
        if let Ok(mut node) = self.node.lock() {
            let p = node.padding();
            ElkPadding::with_values(p.top, p.right, p.bottom, p.left)
        } else {
            ElkPadding::new()
        }
    }

    fn set_padding(&self, padding: ElkPadding) {
        if let Ok(mut node) = self.node.lock() {
            let p = node.padding();
            p.top = padding.top;
            p.right = padding.right;
            p.bottom = padding.bottom;
            p.left = padding.left;
        }
    }

    fn get_margin(&self) -> ElkMargin {
        if let Ok(mut node) = self.node.lock() {
            let m = node.margin();
            ElkMargin::with_values(m.top, m.right, m.bottom, m.left)
        } else {
            ElkMargin::new()
        }
    }

    fn set_margin(&self, margin: ElkMargin) {
        if let Ok(mut node) = self.node.lock() {
            let m = node.margin();
            m.top = margin.top;
            m.right = margin.right;
            m.bottom = margin.bottom;
            m.left = margin.left;
        }
    }
}

/// Adapter wrapping an LPort as a PortAdapter.
#[derive(Clone)]
pub struct LPortAdapter {
    port: LPortRef,
    volatile_id: Cell<i32>,
}

impl LPortAdapter {
    fn new(port: LPortRef) -> Self {
        LPortAdapter {
            port,
            volatile_id: Cell::new(0),
        }
    }
}

impl GraphElementAdapter<LPortRef> for LPortAdapter {
    fn get_size(&self) -> KVector {
        if let Ok(mut port) = self.port.lock() {
            *port.shape().size_ref()
        } else {
            KVector::new()
        }
    }

    fn set_size(&self, size: KVector) {
        if let Ok(mut port) = self.port.lock() {
            let s = port.shape().size();
            s.x = size.x;
            s.y = size.y;
        }
    }

    fn get_position(&self) -> KVector {
        if let Ok(mut port) = self.port.lock() {
            *port.shape().position_ref()
        } else {
            KVector::new()
        }
    }

    fn set_position(&self, pos: KVector) {
        if let Ok(mut port) = self.port.lock() {
            let p = port.shape().position();
            p.x = pos.x;
            p.y = pos.y;
        }
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        if let Ok(mut port) = self.port.lock() {
            port.get_property(prop)
        } else {
            None
        }
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        if let Ok(mut port) = self.port.lock() {
            port.shape().graph_element().properties().has_property(prop)
        } else {
            false
        }
    }

    fn get_volatile_id(&self) -> i32 {
        self.volatile_id.get()
    }

    fn set_volatile_id(&self, id: i32) {
        self.volatile_id.set(id);
    }
}

impl PortAdapter<LPortRef> for LPortAdapter {
    type Label = LLabelRef;
    type LabelAdapter = LLabelAdapter;
    type Edge = LEdgeRef;
    type EdgeAdapter = LEdgeAdapter;

    fn get_side(&self) -> PortSide {
        if let Ok(port) = self.port.lock() {
            port.side()
        } else {
            PortSide::Undefined
        }
    }

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        if let Ok(port) = self.port.lock() {
            port.labels()
                .iter()
                .map(|l| LLabelAdapter::new(l.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_margin(&self) -> ElkMargin {
        if let Ok(mut port) = self.port.lock() {
            let m = port.margin();
            ElkMargin::with_values(m.top, m.right, m.bottom, m.left)
        } else {
            ElkMargin::new()
        }
    }

    fn set_margin(&self, margin: ElkMargin) {
        if let Ok(mut port) = self.port.lock() {
            let m = port.margin();
            m.top = margin.top;
            m.right = margin.right;
            m.bottom = margin.bottom;
            m.left = margin.left;
        }
    }

    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter> {
        if let Ok(port) = self.port.lock() {
            port.incoming_edges()
                .iter()
                .map(|e| LEdgeAdapter::new(e.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter> {
        if let Ok(port) = self.port.lock() {
            port.outgoing_edges()
                .iter()
                .map(|e| LEdgeAdapter::new(e.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn has_compound_connections(&self) -> bool {
        // In the layered algorithm, compound connections are determined by the INSIDE_CONNECTIONS
        // property which is set during graph import.
        if let Ok(mut port) = self.port.lock() {
            port.get_property(InternalProperties::INSIDE_CONNECTIONS)
                .unwrap_or(false)
        } else {
            false
        }
    }
}

/// Adapter wrapping an LLabel as a LabelAdapter.
#[derive(Clone)]
pub struct LLabelAdapter {
    label: LLabelRef,
    volatile_id: Cell<i32>,
}

impl LLabelAdapter {
    fn new(label: LLabelRef) -> Self {
        LLabelAdapter {
            label,
            volatile_id: Cell::new(0),
        }
    }
}

impl GraphElementAdapter<LLabelRef> for LLabelAdapter {
    fn get_size(&self) -> KVector {
        if let Ok(mut label) = self.label.lock() {
            *label.shape().size_ref()
        } else {
            KVector::new()
        }
    }

    fn set_size(&self, size: KVector) {
        if let Ok(mut label) = self.label.lock() {
            let s = label.shape().size();
            s.x = size.x;
            s.y = size.y;
        }
    }

    fn get_position(&self) -> KVector {
        if let Ok(mut label) = self.label.lock() {
            *label.shape().position_ref()
        } else {
            KVector::new()
        }
    }

    fn set_position(&self, pos: KVector) {
        if let Ok(mut label) = self.label.lock() {
            let p = label.shape().position();
            p.x = pos.x;
            p.y = pos.y;
        }
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        if let Ok(mut label) = self.label.lock() {
            label.get_property(prop)
        } else {
            None
        }
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        if let Ok(mut label) = self.label.lock() {
            label.shape().graph_element().properties().has_property(prop)
        } else {
            false
        }
    }

    fn get_volatile_id(&self) -> i32 {
        self.volatile_id.get()
    }

    fn set_volatile_id(&self, id: i32) {
        self.volatile_id.set(id);
    }
}

impl LabelAdapter<LLabelRef> for LLabelAdapter {
    fn get_side(&self) -> LabelSide {
        if let Ok(mut label) = self.label.lock() {
            label
                .get_property(LabelSide::LABEL_SIDE)
                .unwrap_or(LabelSide::Unknown)
        } else {
            LabelSide::Unknown
        }
    }

    fn get_text(&self) -> String {
        if let Ok(label) = self.label.lock() {
            label.text().to_string()
        } else {
            String::new()
        }
    }
}

/// Adapter wrapping an LEdge as an EdgeAdapter.
#[derive(Clone)]
pub struct LEdgeAdapter {
    edge: LEdgeRef,
}

impl LEdgeAdapter {
    fn new(edge: LEdgeRef) -> Self {
        LEdgeAdapter { edge }
    }
}

impl EdgeAdapter<LEdgeRef> for LEdgeAdapter {
    type Label = LLabelRef;
    type LabelAdapter = LLabelAdapter;

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        if let Ok(edge) = self.edge.lock() {
            edge.labels()
                .iter()
                .map(|l| LLabelAdapter::new(l.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }
}
