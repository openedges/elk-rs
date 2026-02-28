use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::{
    index_of_arc, remove_arc, LEdgeRef, LLabelRef, LMargin, LNodeRef, LNodeWeak, LPortRef, LShape,
};

pub struct LPort {
    self_ref: Weak<Mutex<LPort>>,
    shape: LShape,
    owner: Option<LNodeWeak>,
    side: PortSide,
    anchor: KVector,
    explicitly_supplied_port_anchor: bool,
    margin: LMargin,
    labels: Vec<LLabelRef>,
    incoming_edges: Vec<LEdgeRef>,
    outgoing_edges: Vec<LEdgeRef>,
    connected_to_external_nodes: bool,
}

impl LPort {
    pub fn new() -> LPortRef {
        Arc::new_cyclic(|weak| {
            Mutex::new(LPort {
                self_ref: weak.clone(),
                shape: LShape::new(),
                owner: None,
                side: PortSide::Undefined,
                anchor: KVector::new(),
                explicitly_supplied_port_anchor: false,
                margin: LMargin::new(),
                labels: Vec::new(),
                incoming_edges: Vec::new(),
                outgoing_edges: Vec::new(),
                connected_to_external_nodes: true,
            })
        })
    }

    pub fn shape(&mut self) -> &mut LShape {
        &mut self.shape
    }

    pub fn node(&self) -> Option<LNodeRef> {
        self.owner.as_ref().and_then(|owner| owner.upgrade())
    }

    pub fn set_node(port: &LPortRef, node: Option<LNodeRef>) {
        let current_owner = port.lock().ok().and_then(|port| port.node());
        if let (Some(current_owner), Some(new_owner)) = (&current_owner, &node) {
            if Arc::ptr_eq(current_owner, new_owner) {
                return;
            }
        }

        if let Some(current_owner) = current_owner {
            if let Ok(mut owner) = current_owner.lock() {
                remove_arc(owner.ports_mut(), port);
            }
        }

        {
            if let Ok(mut port_guard) = port.lock() {
                port_guard.owner = node.as_ref().map(Arc::downgrade);
            }
        }

        if let Some(new_owner) = node {
            if let Ok(mut owner) = new_owner.lock() {
                owner.ports_mut().push(port.clone());
            }
        }
    }

    pub fn side(&self) -> PortSide {
        self.side
    }

    pub fn set_side(&mut self, side: PortSide) {
        self.side = side;
        if !self.explicitly_supplied_port_anchor {
            let size = self.shape.size_ref();
            match self.side {
                PortSide::North => {
                    self.anchor.x = size.x / 2.0;
                    self.anchor.y = 0.0;
                }
                PortSide::East => {
                    self.anchor.x = size.x;
                    self.anchor.y = size.y / 2.0;
                }
                PortSide::South => {
                    self.anchor.x = size.x / 2.0;
                    self.anchor.y = size.y;
                }
                PortSide::West => {
                    self.anchor.x = 0.0;
                    self.anchor.y = size.y / 2.0;
                }
                PortSide::Undefined => {}
            }
        }
    }

    pub fn anchor(&mut self) -> &mut KVector {
        &mut self.anchor
    }

    pub fn anchor_ref(&self) -> &KVector {
        &self.anchor
    }

    pub fn is_explicitly_supplied_port_anchor(&self) -> bool {
        self.explicitly_supplied_port_anchor
    }

    pub fn set_explicitly_supplied_port_anchor(&mut self, fixed: bool) {
        self.explicitly_supplied_port_anchor = fixed;
    }

    pub fn absolute_anchor(&self) -> Option<KVector> {
        let owner = self.node()?;
        let mut owner_guard = owner.lock().ok()?;
        let mut sum = KVector::new();
        let owner_pos = owner_guard.shape().position_ref();
        let port_pos = self.shape.position_ref();
        sum.x = owner_pos.x + port_pos.x + self.anchor.x;
        sum.y = owner_pos.y + port_pos.y + self.anchor.y;
        Some(sum)
    }

    pub fn margin(&mut self) -> &mut LMargin {
        &mut self.margin
    }

    pub fn labels(&self) -> &Vec<LLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<LLabelRef> {
        &mut self.labels
    }

    pub fn name(&self) -> Option<String> {
        self.labels
            .first()
            .and_then(|label| label.lock().ok().map(|label| label.text().to_owned()))
    }

    pub fn degree(&self) -> usize {
        self.incoming_edges.len() + self.outgoing_edges.len()
    }

    pub fn net_flow(&self) -> isize {
        self.incoming_edges.len() as isize - self.outgoing_edges.len() as isize
    }

    pub fn incoming_edges(&self) -> &Vec<LEdgeRef> {
        &self.incoming_edges
    }

    pub fn incoming_edges_mut(&mut self) -> &mut Vec<LEdgeRef> {
        &mut self.incoming_edges
    }

    pub fn outgoing_edges(&self) -> &Vec<LEdgeRef> {
        &self.outgoing_edges
    }

    pub fn outgoing_edges_mut(&mut self) -> &mut Vec<LEdgeRef> {
        &mut self.outgoing_edges
    }

    pub fn connected_edges(&self) -> Vec<LEdgeRef> {
        let mut edges = Vec::with_capacity(self.incoming_edges.len() + self.outgoing_edges.len());
        edges.extend(self.incoming_edges.iter().cloned());
        edges.extend(self.outgoing_edges.iter().cloned());
        edges
    }

    pub fn is_connected_to_external_nodes(&self) -> bool {
        self.connected_to_external_nodes
    }

    pub fn set_connected_to_external_nodes(&mut self, connected: bool) {
        self.connected_to_external_nodes = connected;
    }

    pub fn predecessor_ports(&self) -> Vec<LPortRef> {
        self.incoming_edges
            .iter()
            .filter_map(|edge| edge.lock().ok().and_then(|edge| edge.source()))
            .collect()
    }

    pub fn successor_ports(&self) -> Vec<LPortRef> {
        self.outgoing_edges
            .iter()
            .filter_map(|edge| edge.lock().ok().and_then(|edge| edge.target()))
            .collect()
    }

    pub fn connected_ports(&self) -> Vec<LPortRef> {
        let mut ports = self.predecessor_ports();
        ports.extend(self.successor_ports());
        ports
    }

    pub fn index(&self) -> Option<usize> {
        let port_ref = self.self_ref.upgrade()?;
        let owner = self.owner.as_ref()?.upgrade()?;
        let owner_guard = owner.lock().ok()?;
        index_of_arc(owner_guard.ports(), &port_ref)
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
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

    pub fn designation(&mut self) -> String {
        if let Some(label) = self.labels.first() {
            if let Ok(label_guard) = label.lock() {
                if !label_guard.text().is_empty() {
                    return label_guard.text().to_owned();
                }
            }
        }
        if let Some(id) = self.shape.graph_element().get_designation() {
            return id;
        }
        self.index()
            .map(|idx| idx.to_string())
            .unwrap_or_else(|| "-1".to_owned())
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&mut self) -> String {
        let mut result = String::new();
        result.push_str("p_");
        result.push_str(&self.designation());

        let self_ref = self.self_ref.upgrade();
        if let Some(owner) = self.node() {
            if let Ok(mut owner_guard) = owner.lock() {
                result.push('[');
                result.push_str(&owner_guard.to_string());
                result.push(']');
            }
        }

        if self.incoming_edges.len() == 1 && self.outgoing_edges.is_empty() {
            if let Some(edge) = self.incoming_edges.first() {
                if let Ok(edge_guard) = edge.lock() {
                    if let Some(source) = edge_guard.source() {
                        if self_ref
                            .as_ref()
                            .map(|self_ref| !Arc::ptr_eq(&source, self_ref))
                            .unwrap_or(true)
                        {
                            if let Ok(mut source_guard) = source.lock() {
                                result.push_str(" << ");
                                result.push_str(&source_guard.designation());
                                if let Some(source_owner) = source_guard.node() {
                                    if let Ok(mut source_owner_guard) = source_owner.lock() {
                                        result.push('[');
                                        result.push_str(&source_owner_guard.to_string());
                                        result.push(']');
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.incoming_edges.is_empty() && self.outgoing_edges.len() == 1 {
            if let Some(edge) = self.outgoing_edges.first() {
                if let Ok(edge_guard) = edge.lock() {
                    if let Some(target) = edge_guard.target() {
                        if self_ref
                            .as_ref()
                            .map(|self_ref| !Arc::ptr_eq(&target, self_ref))
                            .unwrap_or(true)
                        {
                            if let Ok(mut target_guard) = target.lock() {
                                result.push_str(" >> ");
                                result.push_str(&target_guard.designation());
                                if let Some(target_owner) = target_guard.node() {
                                    if let Ok(mut target_owner_guard) = target_owner.lock() {
                                        result.push('[');
                                        result.push_str(&target_owner_guard.to_string());
                                        result.push(']');
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }
}
