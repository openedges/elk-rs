use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraphRef, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;

pub struct LayerSweepTypeDecider {
    node_info: Vec<Vec<NodeInfo>>,
    l_graph: LGraphRef,
    parent: Option<LNodeRef>,
    has_parent: bool,
    cross_min_deterministic: bool,
}

#[derive(Clone, Default)]
struct NodeInfo {
    connected_edges: i32,
    hierarchical_influence: i32,
    random_influence: i32,
}

impl NodeInfo {
    fn transfer(&mut self, other: &NodeInfo) {
        self.hierarchical_influence += other.hierarchical_influence;
        self.random_influence += other.random_influence;
        self.connected_edges += other.connected_edges;
    }
}

impl LayerSweepTypeDecider {
    pub fn new(
        l_graph: LGraphRef,
        parent: Option<LNodeRef>,
        has_parent: bool,
        cross_min_deterministic: bool,
    ) -> Self {
        LayerSweepTypeDecider {
            node_info: Vec::new(),
            l_graph,
            parent,
            has_parent,
            cross_min_deterministic,
        }
    }

    pub fn use_bottom_up(&mut self, node_order: &[Vec<LNodeRef>]) -> bool {
        let boundary = self
            .l_graph
            .lock_ok()
            .and_then(|mut graph_guard| {
                graph_guard
                    .get_property(LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS)
            })
            .unwrap_or(0.0);
        self.use_bottom_up_with_boundary(node_order, boundary)
    }

    pub fn use_bottom_up_with_boundary(
        &mut self,
        node_order: &[Vec<LNodeRef>],
        boundary: f64,
    ) -> bool {
        if self.bottom_up_forced(boundary)
            || self.root_node()
            || self.fixed_port_order()
            || self.fewer_than_two_in_out_edges()
        {
            return true;
        }

        if self.cross_min_deterministic {
            return false;
        }

        let mut paths_to_random = 0;
        let mut paths_to_hierarchical = 0;
        let mut ns_port_dummies: Vec<LNodeRef> = Vec::new();

        for layer in node_order {
            for node in layer {
                if self.is_north_south_dummy(node) {
                    ns_port_dummies.push(node.clone());
                    continue;
                }

                let is_external = self.is_external_port_dummy(node);
                let is_eastern_dummy = self.is_eastern_dummy(node);
                let has_no_western = self.has_no_western_ports(node);
                let has_no_eastern = self.has_no_eastern_ports(node);
                let current_info = {
                    let current_info = self.node_info_for(node);
                    if is_external {
                        current_info.hierarchical_influence = 1;
                        if is_eastern_dummy {
                            paths_to_hierarchical += current_info.connected_edges;
                        }
                    } else if has_no_western {
                        current_info.random_influence = 1;
                    } else if has_no_eastern {
                        paths_to_random += current_info.connected_edges;
                    }
                    current_info.clone()
                };

                let outgoing_edges = node
                    .lock().outgoing_edges();
                for edge in outgoing_edges {
                    paths_to_random += current_info.random_influence;
                    paths_to_hierarchical += current_info.hierarchical_influence;
                    self.transfer_info_to_target(&current_info, &edge);
                }

                let mut north_south_ports = Vec::new();
                {
                    let mut node_guard = node.lock();
                    north_south_ports.extend(node_guard.port_side_view(PortSide::North));
                    north_south_ports.extend(node_guard.port_side_view(PortSide::South));
                }
                for port in north_south_ports {
                    if let Some(ns_dummy) = port.lock_ok().and_then(|mut port_guard| {
                        port_guard.get_property(InternalProperties::PORT_DUMMY)
                    }) {
                        paths_to_random += current_info.random_influence;
                        paths_to_hierarchical += current_info.hierarchical_influence;
                        self.transfer_info_to(&current_info, &ns_dummy);
                    }
                }
            }

            for node in ns_port_dummies.drain(..) {
                let current_info = {
                    let current_info = self.node_info_for(&node);
                    current_info.clone()
                };
                let outgoing_edges = node
                    .lock().outgoing_edges();
                for edge in outgoing_edges {
                    paths_to_random += current_info.random_influence;
                    paths_to_hierarchical += current_info.hierarchical_influence;
                    self.transfer_info_to_target(&current_info, &edge);
                }
            }
        }

        let all_paths = (paths_to_random + paths_to_hierarchical) as f64;
        let normalized = if all_paths == 0.0 {
            f64::INFINITY
        } else {
            (paths_to_random - paths_to_hierarchical) as f64 / all_paths
        };
        normalized >= boundary
    }

    fn fixed_port_order(&self) -> bool {
        let Some(parent) = self.parent.as_ref() else {
            return false;
        };
        parent
            .lock_ok()
            .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            .map(|constraints| constraints.is_order_fixed())
            .unwrap_or(false)
    }

    fn transfer_info_to_target(&mut self, current_info: &NodeInfo, edge: &LEdgeRef) {
        if let Some(target) = target_node(edge) {
            self.transfer_info_to(current_info, &target);
        }
    }

    fn transfer_info_to(&mut self, current_info: &NodeInfo, target: &LNodeRef) {
        let target_info = self.node_info_for(target);
        target_info.transfer(current_info);
        target_info.connected_edges += 1;
    }

    fn fewer_than_two_in_out_edges(&self) -> bool {
        let Some(parent) = self.parent.as_ref() else {
            return true;
        };
        let east = parent
            .lock_ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
            .unwrap_or_default();
        let west = parent
            .lock_ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
            .unwrap_or_default();
        east.len() < 2 && west.len() < 2
    }

    fn root_node(&self) -> bool {
        !self.has_parent
    }

    fn bottom_up_forced(&self, boundary: f64) -> bool {
        boundary < -1.0
    }

    fn has_no_eastern_ports(&self, node: &LNodeRef) -> bool {
        let ports = node
            .lock_ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
            .unwrap_or_default();
        ports.is_empty()
            || !ports.iter().any(|port| {
                port.lock_ok()
                    .map(|port_guard| !port_guard.connected_edges().is_empty())
                    .unwrap_or(false)
            })
    }

    fn has_no_western_ports(&self, node: &LNodeRef) -> bool {
        let ports = node
            .lock_ok()
            .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
            .unwrap_or_default();
        ports.is_empty()
            || !ports.iter().any(|port| {
                port.lock_ok()
                    .map(|port_guard| !port_guard.connected_edges().is_empty())
                    .unwrap_or(false)
            })
    }

    fn is_external_port_dummy(&self, node: &LNodeRef) -> bool {
        node.lock_ok()
            .map(|node_guard| node_guard.node_type() == NodeType::ExternalPort)
            .unwrap_or(false)
    }

    fn is_north_south_dummy(&self, node: &LNodeRef) -> bool {
        node.lock_ok()
            .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
            .unwrap_or(false)
    }

    fn is_eastern_dummy(&self, node: &LNodeRef) -> bool {
        origin_port(node)
            .map(|port| port.lock().side())
            .map(|side| side == PortSide::East)
            .unwrap_or(false)
    }

    fn node_info_for(&mut self, node: &LNodeRef) -> &mut NodeInfo {
        let layer_id = layer_id(node).unwrap_or(0);
        let node_id = node_id(node).unwrap_or(0);
        if self.node_info.len() <= layer_id {
            self.node_info.resize(layer_id + 1, Vec::new());
        }
        if self.node_info[layer_id].len() <= node_id {
            self.node_info[layer_id].resize(node_id + 1, NodeInfo::default());
        }
        &mut self.node_info[layer_id][node_id]
    }
}

impl IInitializable for LayerSweepTypeDecider {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Some(first_node) = node_order.get(layer_index).and_then(|layer| layer.first()) {
            if let Some(layer) = first_node
                .lock().layer()
            {
                {
                    let mut layer_guard = layer.lock();
                    layer_guard.graph_element().id = layer_index as i32;
                }
            }
        }
        if self.node_info.len() <= layer_index {
            self.node_info.resize(layer_index + 1, Vec::new());
        }
        if let Some(layer_nodes) = node_order.get(layer_index) {
            self.node_info[layer_index] = vec![NodeInfo::default(); layer_nodes.len()];
        }
    }

    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        let Some(node) = node_order
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
        else {
            return;
        };
        {
            let mut node_guard = node.lock();
            node_guard.shape().graph_element().id = node_index as i32;
        }
        if self.node_info.len() <= layer_index {
            self.node_info.resize(layer_index + 1, Vec::new());
        }
        if self.node_info[layer_index].len() <= node_index {
            self.node_info[layer_index].resize(node_index + 1, NodeInfo::default());
        }
        self.node_info[layer_index][node_index] = NodeInfo::default();
    }
}

fn target_node(edge: &LEdgeRef) -> Option<LNodeRef> {
    edge.lock().target()
        .and_then(|port| port.lock().node())
}

fn origin_port(node: &LNodeRef) -> Option<LPortRef> {
    node.lock_ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            Origin::LPort(port) => Some(port),
            _ => None,
        })
}

fn layer_id(node: &LNodeRef) -> Option<usize> {
    node.lock().layer()
        .and_then(|layer| {
            layer
                .lock_ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
}

fn node_id(node: &LNodeRef) -> Option<usize> {
    node.lock_ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
}
