use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;

use crate::org::eclipse::elk::alg::layered::options::{
    InteractiveReferencePoint, InternalProperties, LayerConstraint, LayeredOptions, Origin,
    PortType,
};

use super::{
    index_of_arc, remove_arc, LEdgeRef, LGraphRef, LGraphWeak, LLabelRef, LMargin, LNodeRef,
    LPadding, LPortRef, LShape, LayerRef, LayerWeak,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodeType {
    Normal,
    LongEdge,
    ExternalPort,
    NorthSouthPort,
    Label,
    BreakingPoint,
    Placeholder,
    NonshiftingPlaceholder,
}

impl NodeType {
    pub const COUNT: usize = 8;
    pub const VALUES: [NodeType; Self::COUNT] = [
        NodeType::Normal,
        NodeType::LongEdge,
        NodeType::ExternalPort,
        NodeType::NorthSouthPort,
        NodeType::Label,
        NodeType::BreakingPoint,
        NodeType::Placeholder,
        NodeType::NonshiftingPlaceholder,
    ];

    pub fn ordinal(&self) -> usize {
        match self {
            NodeType::Normal => 0,
            NodeType::LongEdge => 1,
            NodeType::ExternalPort => 2,
            NodeType::NorthSouthPort => 3,
            NodeType::Label => 4,
            NodeType::BreakingPoint => 5,
            NodeType::Placeholder => 6,
            NodeType::NonshiftingPlaceholder => 7,
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            NodeType::ExternalPort => "#cc99cc",
            NodeType::LongEdge => "#eaed00",
            NodeType::NorthSouthPort => "#0034de",
            NodeType::Label => "#75c3c3",
            NodeType::BreakingPoint => "#eeeeff",
            _ => "#eeeeee",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Normal => "NORMAL",
            NodeType::LongEdge => "LONG_EDGE",
            NodeType::ExternalPort => "EXTERNAL_PORT",
            NodeType::NorthSouthPort => "NORTH_SOUTH_PORT",
            NodeType::Label => "LABEL",
            NodeType::BreakingPoint => "BREAKING_POINT",
            NodeType::Placeholder => "PLACEHOLDER",
            NodeType::NonshiftingPlaceholder => "NONSHIFTING_PLACEHOLDER",
        }
    }
}

pub struct LNode {
    self_ref: Weak<Mutex<LNode>>,
    shape: LShape,
    graph: Option<LGraphWeak>,
    layer: Option<LayerWeak>,
    node_type: NodeType,
    ports: Vec<LPortRef>,
    labels: Vec<LLabelRef>,
    nested_graph: Option<LGraphRef>,
    margin: LMargin,
    padding: LPadding,
    port_side_indices: Option<[Option<Pair<usize, usize>>; 5]>,
    port_sides_cached: bool,
}

impl LNode {
    pub fn new(graph: &LGraphRef) -> LNodeRef {
        Arc::new_cyclic(|weak| {
            Mutex::new(LNode {
                self_ref: weak.clone(),
                shape: LShape::new(),
                graph: Some(Arc::downgrade(graph)),
                layer: None,
                node_type: NodeType::Normal,
                ports: Vec::new(),
                labels: Vec::new(),
                nested_graph: None,
                margin: LMargin::new(),
                padding: LPadding::new(),
                port_side_indices: None,
                port_sides_cached: false,
            })
        })
    }

    pub fn shape(&mut self) -> &mut LShape {
        &mut self.shape
    }

    pub fn layer(&self) -> Option<LayerRef> {
        self.layer.as_ref().and_then(|layer| layer.upgrade())
    }

    pub fn set_layer(node: &LNodeRef, layer: Option<LayerRef>) {
        let current_layer = node.lock().layer();

        if let Some(current_layer) = current_layer {
            let mut layer = current_layer.lock();
            remove_arc(layer.nodes_mut(), node);
        }

        {
            let mut node_guard = node.lock();
            node_guard.layer = layer.as_ref().map(Arc::downgrade);
        }

        if let Some(layer) = layer {
            let mut layer_guard = layer.lock();
            layer_guard.nodes_mut().push(node.clone());
        }
    }

    pub fn set_layer_at_index(node: &LNodeRef, index: usize, layer: Option<LayerRef>) {
        if let Some(layer_ref) = &layer {
            let size = layer_ref.lock().nodes().len();
            if index > size {
                panic!("index must be >= 0 and <= layer node count");
            }
        }

        let current_layer = node.lock().layer();
        if let Some(current_layer) = current_layer {
            let mut layer = current_layer.lock();
            remove_arc(layer.nodes_mut(), node);
        }

        {
            let mut node_guard = node.lock();
            node_guard.layer = layer.as_ref().map(Arc::downgrade);
        }

        if let Some(layer) = layer {
            let mut layer_guard = layer.lock();
            if index > layer_guard.nodes().len() {
                panic!("index must be >= 0 and <= layer node count");
            }
            layer_guard.nodes_mut().insert(index, node.clone());
        }
    }

    pub fn graph(&self) -> Option<LGraphRef> {
        if let Some(graph) = self.graph.as_ref().and_then(|graph| graph.upgrade()) {
            return Some(graph);
        }
        self.layer()
            .and_then(|layer| layer.lock().graph())
    }

    pub fn set_graph(&mut self, graph: &LGraphRef) {
        if self.layer.is_some() {
            panic!("layer already assigned");
        }
        self.graph = Some(Arc::downgrade(graph));
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    pub fn set_node_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }

    pub fn ports(&self) -> &Vec<LPortRef> {
        &self.ports
    }

    pub fn ports_mut(&mut self) -> &mut Vec<LPortRef> {
        &mut self.ports
    }

    pub fn ports_by_type(&self, port_type: PortType) -> Vec<LPortRef> {
        match port_type {
            PortType::Input => self
                .ports
                .iter()
                .filter(|port| !port.lock().incoming_edges().is_empty())
                .cloned()
                .collect(),
            PortType::Output => self
                .ports
                .iter()
                .filter(|port| !port.lock().outgoing_edges().is_empty())
                .cloned()
                .collect(),
            PortType::Undefined => Vec::new(),
        }
    }

    pub fn ports_by_side(&self, side: PortSide) -> Vec<LPortRef> {
        self.ports
            .iter()
            .filter(|port| port.lock().side() == side)
            .cloned()
            .collect()
    }

    pub fn port_side_view(&mut self, side: PortSide) -> Vec<LPortRef> {
        if self.port_sides_cached && self.port_side_indices.is_none() {
            self.find_port_indices();
        }
        if self.port_sides_cached {
            if let Some(indices) = self
                .port_side_indices
                .as_ref()
                .and_then(|arr| arr[side as usize].as_ref())
            {
                let slice = &self.ports[indices.first..indices.second];
                let matches = slice.iter().all(|port| port.lock().side() == side);
                if matches {
                    return slice.to_vec();
                }
            }
        }

        // Cache can be stale if port sides changed after caching.
        self.ports
            .iter()
            .filter(|port| port.lock().side() == side)
            .cloned()
            .collect()
    }

    pub fn ports_by_type_and_side(&self, port_type: PortType, side: PortSide) -> Vec<LPortRef> {
        self.ports
            .iter()
            .filter(|port| {
                let port_guard = port.lock();
                if port_guard.side() != side {
                    return false;
                }
                match port_type {
                    PortType::Input => !port_guard.incoming_edges().is_empty(),
                    PortType::Output => !port_guard.outgoing_edges().is_empty(),
                    PortType::Undefined => false,
                }
            })
            .cloned()
            .collect()
    }

    pub fn incoming_edges(&self) -> Vec<LEdgeRef> {
        let mut edges = Vec::new();
        for port in &self.ports {
            let port_guard = port.lock();
            edges.extend(port_guard.incoming_edges().iter().cloned());
        }
        edges
    }

    pub fn outgoing_edges(&self) -> Vec<LEdgeRef> {
        let mut edges = Vec::new();
        for port in &self.ports {
            let port_guard = port.lock();
            edges.extend(port_guard.outgoing_edges().iter().cloned());
        }
        edges
    }

    pub fn connected_edges(&self) -> Vec<LEdgeRef> {
        let mut edges = Vec::new();
        for port in &self.ports {
            let port_guard = port.lock();
            edges.extend(port_guard.connected_edges().iter().cloned());
        }
        edges
    }

    pub fn labels(&self) -> &Vec<LLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<LLabelRef> {
        &mut self.labels
    }

    pub fn nested_graph(&self) -> Option<LGraphRef> {
        self.nested_graph.clone()
    }

    pub fn set_nested_graph(&mut self, graph: Option<LGraphRef>) {
        self.nested_graph = graph;
    }

    pub fn margin(&mut self) -> &mut LMargin {
        &mut self.margin
    }

    pub fn padding(&mut self) -> &mut LPadding {
        &mut self.padding
    }

    pub fn index(&self) -> Option<usize> {
        let node_ref = self.self_ref.upgrade()?;
        let layer = self.layer()?;
        let layer_guard = layer.lock();
        index_of_arc(layer_guard.nodes(), &node_ref)
    }

    pub fn border_to_content_area_coordinates(&mut self, horizontal: bool, vertical: bool) {
        let graph = self.graph().expect("node must be assigned to a graph");
        let graph_guard = graph.lock();        let padding = graph_guard.padding_ref();
        let offset = graph_guard.offset_ref();
        let pos = self.shape.position();

        if horizontal {
            pos.x -= padding.left + offset.x;
        }

        if vertical {
            pos.y -= padding.top + offset.y;
        }
    }

    pub fn interactive_reference_point(&mut self) -> Option<KVector> {
        let graph = self.graph()?;
        let graph_guard = graph.lock();
        let reference = graph_guard
            .get_property(LayeredOptions::INTERACTIVE_REFERENCE_POINT)
            .unwrap_or(InteractiveReferencePoint::Center);

        match reference {
            InteractiveReferencePoint::Center => {
                let pos = self.shape.position_ref();
                let size = self.shape.size_ref();
                Some(KVector::with_values(
                    pos.x + size.x / 2.0,
                    pos.y + size.y / 2.0,
                ))
            }
            InteractiveReferencePoint::TopLeft => {
                Some(KVector::from_vector(self.shape.position_ref()))
            }
        }
    }

    pub fn cache_port_sides(&mut self) {
        self.port_sides_cached = true;
        self.find_port_indices();
    }

    fn find_port_indices(&mut self) {
        let mut indices: [Option<Pair<usize, usize>>; 5] = [const { None }; 5];
        if self.ports.is_empty() {
            self.port_side_indices = Some(indices);
            return;
        }

        let mut first_index = 0usize;
        let mut current_side = PortSide::North;
        let mut current_index = 0usize;
        for port in &self.ports {
            let side = port.lock().side();
            if side != current_side {
                if first_index != current_index {
                    indices[current_side as usize] = Some(Pair::of(first_index, current_index));
                }
                current_side = side;
                first_index = current_index;
            }
            current_index += 1;
        }
        indices[current_side as usize] = Some(Pair::of(first_index, current_index));
        self.port_side_indices = Some(indices);
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        self.shape.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.shape.set_property(property, value);
    }

    // --- Typed property accessors (read-only, &self) ---

    pub fn origin(&self) -> Option<Origin> {
        self.shape.get_property(InternalProperties::ORIGIN)
    }

    pub fn port_constraints_prop(&self) -> PortConstraints {
        self.shape
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined)
    }

    pub fn ext_port_side(&self) -> PortSide {
        self.shape
            .get_property(InternalProperties::EXT_PORT_SIDE)
            .unwrap_or(PortSide::Undefined)
    }

    pub fn model_order(&self) -> Option<i32> {
        self.shape.get_property(InternalProperties::MODEL_ORDER)
    }

    pub fn layer_constraint(&self) -> LayerConstraint {
        self.shape
            .get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            .unwrap_or(LayerConstraint::None)
    }

    pub fn port_ratio_or_position(&self) -> Option<f64> {
        self.shape
            .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
    }

    pub fn in_layer_layout_unit(&self) -> Option<LNodeRef> {
        self.shape
            .get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
    }

    pub fn designation(&self) -> String {
        if let Some(label) = self.labels.first() {
            let label_guard = label.lock();
            if !label_guard.text().is_empty() {
                return label_guard.text().to_owned();
            }
        }

        if let Some(id) = self.shape.graph_element_ref().get_designation() {
            return id;
        }

        self.index()
            .map(|idx| idx.to_string())
            .unwrap_or_else(|| "-1".to_owned())
    }

    pub fn is_inline_edge_label(&mut self) -> bool {
        if self.node_type != NodeType::Label {
            return false;
        }
        let labels = self
            .get_property(InternalProperties::REPRESENTED_LABELS)
            .unwrap_or_default();
        if labels.is_empty() {
            return false;
        }
        labels.iter().all(|label| {
            label
                .lock()
                .get_property(LayeredOptions::EDGE_LABELS_INLINE)
                .unwrap_or(false)
        })
    }

}

impl fmt::Display for LNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("n")?;
        if self.node_type != NodeType::Normal {
            write!(f, "({})", self.node_type.name().to_lowercase())?;
        }
        f.write_str("_")?;
        f.write_str(&self.designation())
    }
}
