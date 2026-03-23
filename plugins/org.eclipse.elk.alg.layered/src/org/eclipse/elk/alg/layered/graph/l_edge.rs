use std::fmt;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, Origin, PortType,
};

use super::LGraphUtil;
use super::{
    remove_arc, LEdgeRef, LGraphElement, LGraphRef, LLabelRef, LNodeRef, LPortRef, LPortWeak,
};

pub struct LEdge {
    element: LGraphElement,
    bend_points: KVectorChain,
    source: Option<LPortWeak>,
    target: Option<LPortWeak>,
    labels: Vec<LLabelRef>,
}

impl LEdge {
    pub fn new() -> LEdgeRef {
        Arc::new(Mutex::new(LEdge {
            element: LGraphElement::new(),
            bend_points: KVectorChain::new(),
            source: None,
            target: None,
            labels: Vec::new(),
        }))
    }

    pub fn graph_element(&mut self) -> &mut LGraphElement {
        &mut self.element
    }

    pub fn bend_points(&mut self) -> &mut KVectorChain {
        &mut self.bend_points
    }

    pub fn bend_points_ref(&self) -> &KVectorChain {
        &self.bend_points
    }

    pub fn labels(&self) -> &Vec<LLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<LLabelRef> {
        &mut self.labels
    }

    pub fn source(&self) -> Option<LPortRef> {
        self.source.as_ref().and_then(|source| source.upgrade())
    }

    pub fn target(&self) -> Option<LPortRef> {
        self.target.as_ref().and_then(|target| target.upgrade())
    }

    pub fn set_source(edge: &LEdgeRef, source: Option<LPortRef>) {
        let current_source = edge.lock().source();
        if let Some(current_source) = current_source {
            {
                let mut port = current_source.lock();
                remove_arc(port.outgoing_edges_mut(), edge);
            }
        }

        if let Some(source_ref) = &source {
            {
                let mut port = source_ref.lock();
                port.outgoing_edges_mut().push(edge.clone());
            }
        }

        {
            let mut edge_guard = edge.lock();
            edge_guard.source = source.as_ref().map(Arc::downgrade);
        }
    }

    pub fn set_target(edge: &LEdgeRef, target: Option<LPortRef>) {
        let current_target = edge.lock().target();
        if let Some(current_target) = current_target {
            {
                let mut port = current_target.lock();
                remove_arc(port.incoming_edges_mut(), edge);
            }
        }

        if let Some(target_ref) = &target {
            {
                let mut port = target_ref.lock();
                port.incoming_edges_mut().push(edge.clone());
            }
        }

        {
            let mut edge_guard = edge.lock();
            edge_guard.target = target.as_ref().map(Arc::downgrade);
        }
    }

    pub fn set_target_and_insert_at_index(edge: &LEdgeRef, target: Option<LPortRef>, index: usize) {
        let current_target = edge.lock().target();
        if let Some(current_target) = current_target {
            let mut port = current_target.lock();
            remove_arc(port.incoming_edges_mut(), edge);
        }

        if let Some(target_ref) = &target {
            {
                let mut port = target_ref.lock();
                if index > port.incoming_edges().len() {
                    panic!("index out of bounds");
                }
                port.incoming_edges_mut().insert(index, edge.clone());
            }
        }

        {
            let mut edge_guard = edge.lock();
            edge_guard.target = target.as_ref().map(Arc::downgrade);
        }
    }

    pub fn is_self_loop(&self) -> bool {
        match (self.source(), self.target()) {
            (Some(source), Some(target)) => {
                let source_node = source.lock().node();
                let target_node = target.lock().node();
                if let (Some(source_node), Some(target_node)) = (source_node, target_node) {
                    Arc::ptr_eq(&source_node, &target_node)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn is_in_layer_edge(&self) -> bool {
        if self.is_self_loop() {
            return false;
        }
        let source = self.source();
        let target = self.target();
        if let (Some(source), Some(target)) = (source, target) {
            let source_layer = source
                .lock().node()
                .and_then(|node| node.lock().layer());
            let target_layer = target
                .lock().node()
                .and_then(|node| node.lock().layer());
            if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
                return Arc::ptr_eq(&source_layer, &target_layer);
            }
        }
        false
    }

    pub fn other_port(&self, port: &LPortRef) -> LPortRef {
        if let Some(source) = self.source() {
            if Arc::ptr_eq(&source, port) {
                return self.target().expect("target missing");
            }
        }
        if let Some(target) = self.target() {
            if Arc::ptr_eq(&target, port) {
                return self.source().expect("source missing");
            }
        }
        panic!("port must be either the source port or target port of the edge");
    }

    pub fn other_node(&self, node: &LNodeRef) -> LNodeRef {
        let source = self
            .source()
            .and_then(|port| port.lock().node());
        let target = self
            .target()
            .and_then(|port| port.lock().node());
        if let Some(ref source) = source {
            if Arc::ptr_eq(source, node) {
                return target.clone().expect("target missing");
            }
        }
        if let Some(ref target) = target {
            if Arc::ptr_eq(target, node) {
                return source.clone().expect("source missing");
            }
        }
        panic!("node must either be the source node or target node of the edge");
    }

    pub fn reverse(edge: &LEdgeRef, layered_graph: &LGraphRef, adapt_ports: bool) {
        let (old_source, old_target) = {
            let edge_guard = edge.lock();
            (
                edge_guard.source(),
                edge_guard.target(),
            )
        };

        LEdge::set_source(edge, None);
        LEdge::set_target(edge, None);

        if let Some(old_target) = old_target {
            let use_collector = adapt_ports
                && old_target
                    .lock()
                    .get_property(InternalProperties::INPUT_COLLECT)
                    .unwrap_or(false);
            if use_collector {
                if let Some(node) = old_target.lock().node() {
                    let port = LGraphUtil::provide_collector_port(
                        layered_graph,
                        &node,
                        PortType::Output,
                        PortSide::East,
                    );
                    LEdge::set_source(edge, Some(port));
                }
            } else {
                LEdge::set_source(edge, Some(old_target));
            }
        }

        if let Some(old_source) = old_source {
            let use_collector = adapt_ports
                && old_source
                    .lock()
                    .get_property(InternalProperties::OUTPUT_COLLECT)
                    .unwrap_or(false);
            if use_collector {
                if let Some(node) = old_source.lock().node() {
                    let port = LGraphUtil::provide_collector_port(
                        layered_graph,
                        &node,
                        PortType::Input,
                        PortSide::West,
                    );
                    LEdge::set_target(edge, Some(port));
                }
            } else {
                LEdge::set_target(edge, Some(old_source));
            }
        }

        {
            let mut edge_guard = edge.lock();
            for label in &edge_guard.labels {
                {
                    let mut label_guard = label.lock();
                    let placement = label_guard
                        .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                        .unwrap_or(EdgeLabelPlacement::Center);
                    match placement {
                        EdgeLabelPlacement::Tail => {
                            label_guard.set_property(
                                LayeredOptions::EDGE_LABELS_PLACEMENT,
                                Some(EdgeLabelPlacement::Head),
                            );
                        }
                        EdgeLabelPlacement::Head => {
                            label_guard.set_property(
                                LayeredOptions::EDGE_LABELS_PLACEMENT,
                                Some(EdgeLabelPlacement::Tail),
                            );
                        }
                        _ => {}
                    }
                }
            }

            let reversed = edge_guard
                .get_property(InternalProperties::REVERSED)
                .unwrap_or(false);
            edge_guard.set_property(InternalProperties::REVERSED, Some(!reversed));
            edge_guard.bend_points = KVectorChain::reverse(&edge_guard.bend_points);
        }
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

    pub fn origin(&self) -> Option<Origin> {
        self.element.properties().get_property(InternalProperties::ORIGIN)
    }

    pub fn junction_points(&self) -> Option<KVectorChain> {
        self.element.properties().get_property(LayeredOptions::JUNCTION_POINTS)
    }

    pub fn is_reversed(&self) -> bool {
        self.element
            .properties()
            .get_property(InternalProperties::REVERSED)
            .unwrap_or(false)
    }

    pub fn priority_straightness(&self) -> i32 {
        self.element
            .properties()
            .get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
            .unwrap_or(0)
    }

    pub fn designation(&self) -> Option<String> {
        if let Some(label) = self.labels.first() {
            {
                let label_guard = label.lock();
                if !label_guard.text().is_empty() {
                    return Some(label_guard.text().to_owned());
                }
            }
        }
        self.element.get_designation()
    }

}

impl fmt::Display for LEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("e_")?;
        if let Some(designation) = self.designation() {
            f.write_str(&designation)?;
        }
        if let (Some(source), Some(target)) = (self.source(), self.target()) {
            {
                let source_guard = source.lock();
                f.write_str(" ")?;
                f.write_str(&source_guard.designation())?;
                if let Some(source_node) = source_guard.node() {
                    {
                        let source_node_guard = source_node.lock();
                        write!(f, "[{}]", source_node_guard)?;
                    }
                }
            }
            f.write_str(" -> ")?;
            {
                let target_guard = target.lock();
                f.write_str(&target_guard.designation())?;
                if let Some(target_node) = target_guard.node() {
                    {
                        let target_node_guard = target_node.lock();
                        write!(f, "[{}]", target_node_guard)?;
                    }
                }
            }
        }
        Ok(())
    }
}
