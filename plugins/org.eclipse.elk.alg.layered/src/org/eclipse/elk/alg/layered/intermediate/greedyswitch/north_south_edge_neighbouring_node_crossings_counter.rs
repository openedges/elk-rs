use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::counting::in_north_south_east_west_order;

pub struct NorthSouthEdgeNeighbouringNodeCrossingsCounter {
    upper_lower_crossings: i32,
    lower_upper_crossings: i32,
    port_positions: HashMap<usize, i32>,
    layer: Vec<LNodeRef>,
}

impl NorthSouthEdgeNeighbouringNodeCrossingsCounter {
    pub fn new(nodes: &[LNodeRef]) -> Self {
        let mut counter = NorthSouthEdgeNeighbouringNodeCrossingsCounter {
            upper_lower_crossings: 0,
            lower_upper_crossings: 0,
            port_positions: HashMap::new(),
            layer: nodes.to_vec(),
        };
        counter.initialize_port_positions();
        counter
    }

    pub fn count_crossings(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        self.upper_lower_crossings = 0;
        self.lower_upper_crossings = 0;

        self.process_if_two_north_south_nodes(upper_node, lower_node);
        self.process_if_north_south_long_edge_dummy_crossing(upper_node, lower_node);
        self.process_if_normal_node_with_ns_ports_and_long_edge_dummy(upper_node, lower_node);
    }

    pub fn upper_lower_crossings(&self) -> i32 {
        self.upper_lower_crossings
    }

    pub fn lower_upper_crossings(&self) -> i32 {
        self.lower_upper_crossings
    }

    fn initialize_port_positions(&mut self) {
        let nodes = self.layer.clone();
        for node in nodes {
            self.set_port_ids_on(&node, PortSide::South);
            self.set_port_ids_on(&node, PortSide::North);
        }
    }

    fn set_port_ids_on(&mut self, node: &LNodeRef, side: PortSide) {
        let ports = in_north_south_east_west_order(node, side);
        for (port_id, port) in ports.into_iter().enumerate() {
            self.port_positions.insert(port_ptr_id(&port), port_id as i32);
        }
    }

    fn process_if_two_north_south_nodes(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        if self.is_north_south(upper_node)
            && self.is_north_south(lower_node)
            && !self.have_different_origins(upper_node, lower_node)
        {
            if self.is_north_of_normal_node(upper_node) {
                self.count_crossings_of_two_north_south_dummies(upper_node, lower_node);
            } else {
                self.count_crossings_of_two_north_south_dummies(lower_node, upper_node);
            }
        }
    }

    fn count_crossings_of_two_north_south_dummies(
        &mut self,
        further_from_normal: &LNodeRef,
        closer_to_normal: &LNodeRef,
    ) {
        if self.origin_port_position_of(further_from_normal)
            > self.origin_port_position_of(closer_to_normal)
        {
            let closer_east_ports = ports_on_side(closer_to_normal, PortSide::East);
            self.upper_lower_crossings = closer_east_ports
                .first()
                .and_then(|port| port.lock().ok().map(|port_guard| port_guard.degree() as i32))
                .unwrap_or(0);
            let further_west_ports = ports_on_side(further_from_normal, PortSide::West);
            self.lower_upper_crossings = further_west_ports
                .first()
                .and_then(|port| port.lock().ok().map(|port_guard| port_guard.degree() as i32))
                .unwrap_or(0);
        } else {
            let closer_west_ports = ports_on_side(closer_to_normal, PortSide::West);
            self.upper_lower_crossings = closer_west_ports
                .first()
                .and_then(|port| port.lock().ok().map(|port_guard| port_guard.degree() as i32))
                .unwrap_or(0);
            let further_east_ports = ports_on_side(further_from_normal, PortSide::East);
            self.lower_upper_crossings = further_east_ports
                .first()
                .and_then(|port| port.lock().ok().map(|port_guard| port_guard.degree() as i32))
                .unwrap_or(0);
        }
    }

    fn process_if_north_south_long_edge_dummy_crossing(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
    ) {
        if self.is_north_south(upper_node) && self.is_long_edge_dummy(lower_node) {
            if self.is_north_of_normal_node(upper_node) {
                self.upper_lower_crossings = 1;
            } else {
                self.lower_upper_crossings = 1;
            }
        } else if self.is_north_south(lower_node) && self.is_long_edge_dummy(upper_node) {
            if self.is_north_of_normal_node(lower_node) {
                self.lower_upper_crossings = 1;
            } else {
                self.upper_lower_crossings = 1;
            }
        }
    }

    fn process_if_normal_node_with_ns_ports_and_long_edge_dummy(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
    ) {
        if self.is_normal(upper_node) && self.is_long_edge_dummy(lower_node) {
            self.upper_lower_crossings = self.number_of_north_south_edges(upper_node, PortSide::South);
            self.lower_upper_crossings = self.number_of_north_south_edges(upper_node, PortSide::North);
        }
        if self.is_normal(lower_node) && self.is_long_edge_dummy(upper_node) {
            self.upper_lower_crossings = self.number_of_north_south_edges(lower_node, PortSide::North);
            self.lower_upper_crossings = self.number_of_north_south_edges(lower_node, PortSide::South);
        }
    }

    fn number_of_north_south_edges(&self, node: &LNodeRef, side: PortSide) -> i32 {
        let mut count = 0;
        for port in ports_on_side(node, side) {
            if self.has_connected_north_south_edge(&port) {
                count += 1;
            }
        }
        count
    }

    fn has_connected_north_south_edge(&self, port: &LPortRef) -> bool {
        port.lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY))
            .is_some()
    }

    fn have_different_origins(&self, upper_node: &LNodeRef, lower_node: &LNodeRef) -> bool {
        !self.origin_of(upper_node).is_some_and(|origin| {
            self.origin_of(lower_node)
                .map(|other| Arc::ptr_eq(&origin, &other))
                .unwrap_or(false)
        })
    }

    fn origin_port_position_of(&self, node: &LNodeRef) -> i32 {
        self.origin_port_of(node)
            .and_then(|port| self.port_positions.get(&port_ptr_id(&port)).copied())
            .unwrap_or(0)
    }

    fn origin_port_of(&self, node: &LNodeRef) -> Option<LPortRef> {
        let port = node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.ports().first().cloned());
        let port = port?;
        let origin = port
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
        match origin {
            Some(Origin::LPort(origin_port)) => Some(origin_port),
            _ => None,
        }
    }

    fn is_north_of_normal_node(&self, node: &LNodeRef) -> bool {
        self.origin_port_of(node)
            .and_then(|port| port.lock().ok().map(|port_guard| port_guard.side()))
            .map(|side| side == PortSide::North)
            .unwrap_or(false)
    }

    fn origin_of(&self, node: &LNodeRef) -> Option<LNodeRef> {
        node.lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
            .and_then(|origin| match origin {
                Origin::LNode(node_ref) => Some(node_ref),
                _ => None,
            })
    }

    fn is_long_edge_dummy(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
            .unwrap_or(false)
    }

    fn is_north_south(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
            .unwrap_or(false)
    }

    fn is_normal(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::Normal)
            .unwrap_or(false)
    }
}

fn ports_on_side(node: &LNodeRef, side: PortSide) -> Vec<LPortRef> {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default()
}

fn port_ptr_id(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}
