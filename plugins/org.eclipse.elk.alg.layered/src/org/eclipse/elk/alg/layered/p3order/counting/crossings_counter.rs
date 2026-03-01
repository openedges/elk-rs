use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, LazyLock};

static TRACE_CROSSINGS_BREAKDOWN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSINGS_BREAKDOWN").is_some());

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::counting::{
    in_north_south_east_west_order, BinaryIndexedTree,
};

pub struct CrossingsCounter {
    port_positions: Vec<i32>,
    index_tree: BinaryIndexedTree,
    ends: VecDeque<i32>,
    node_cardinalities: Vec<i32>,
}

impl CrossingsCounter {
    pub fn new(port_positions: Vec<i32>) -> Self {
        CrossingsCounter {
            port_positions,
            index_tree: BinaryIndexedTree::new(0),
            ends: VecDeque::new(),
            node_cardinalities: Vec::new(),
        }
    }

    pub fn count_crossings_between_layers(
        &mut self,
        left_layer: &[LNodeRef],
        right_layer: &[LNodeRef],
    ) -> i32 {
        let ports = self.init_port_positions_counter_clockwise(left_layer, right_layer);
        self.index_tree = BinaryIndexedTree::new(ports.len());
        self.count_crossings_on_ports(&ports)
    }

    pub fn count_in_layer_crossings_on_side(&mut self, nodes: &[LNodeRef], side: PortSide) -> i32 {
        let ports = self.init_port_positions_for_in_layer_crossings(nodes, side);
        self.count_in_layer_crossings_on_ports(&ports)
    }

    pub fn count_north_south_port_crossings_in_layer(&mut self, layer: &[LNodeRef]) -> i32 {
        let ports = self.init_positions_for_north_south_counting(layer);
        self.index_tree = BinaryIndexedTree::new(ports.len());
        self.count_north_south_crossings_on_ports(&ports)
    }

    pub fn count_crossings_between_ports_in_both_orders(
        &mut self,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
    ) -> Pair<i32, i32> {
        let mut ports = self.connected_ports_sorted_by_position(upper_port, lower_port);
        let upper_lower_crossings = self.count_crossings_on_ports(&ports);
        self.index_tree.clear();
        self.switch_ports(upper_port, lower_port);
        ports.sort_by_key(|port| self.position_of(port));
        let lower_upper_crossings = self.count_crossings_on_ports(&ports);
        self.index_tree.clear();
        self.switch_ports(lower_port, upper_port);
        Pair::of(upper_lower_crossings, lower_upper_crossings)
    }

    pub fn count_in_layer_crossings_between_nodes_in_both_orders(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
        side: PortSide,
    ) -> Pair<i32, i32> {
        let mut ports =
            self.connected_in_layer_ports_sorted_by_position(upper_node, lower_node, side);
        let upper_lower_crossings = self.count_in_layer_crossings_on_ports(&ports);
        self.switch_nodes(upper_node, lower_node, side);
        self.index_tree.clear();
        ports.sort_by_key(|port| self.position_of(port));
        let lower_upper_crossings = self.count_in_layer_crossings_on_ports(&ports);
        self.switch_nodes(lower_node, upper_node, side);
        self.index_tree.clear();
        Pair::of(upper_lower_crossings, lower_upper_crossings)
    }

    pub fn init_for_counting_between(&mut self, left_layer: &[LNodeRef], right_layer: &[LNodeRef]) {
        let ports = self.init_port_positions_counter_clockwise(left_layer, right_layer);
        self.index_tree = BinaryIndexedTree::new(ports.len());
    }

    pub fn init_port_positions_for_in_layer_crossings(
        &mut self,
        nodes: &[LNodeRef],
        side: PortSide,
    ) -> Vec<LPortRef> {
        let mut ports = Vec::new();
        self.init_positions(nodes, &mut ports, side, true, true);
        self.index_tree = BinaryIndexedTree::new(ports.len());
        ports
    }

    pub fn switch_ports(&mut self, top_port: &LPortRef, bottom_port: &LPortRef) {
        let top_index = port_id(top_port);
        let bottom_index = port_id(bottom_port);
        if top_index >= self.port_positions.len() || bottom_index >= self.port_positions.len() {
            return;
        }
        self.port_positions.swap(top_index, bottom_index);
    }

    pub fn switch_nodes(&mut self, was_upper: &LNodeRef, was_lower: &LNodeRef, side: PortSide) {
        let upper_id = node_id(was_upper);
        let lower_id = node_id(was_lower);
        let upper_shift = *self.node_cardinalities.get(lower_id).unwrap_or(&0);
        let lower_shift = *self.node_cardinalities.get(upper_id).unwrap_or(&0);

        for port in in_north_south_east_west_order(was_upper, side) {
            let idx = port_id(&port);
            if idx < self.port_positions.len() {
                self.port_positions[idx] = self.position_of(&port) + upper_shift;
            }
        }

        for port in in_north_south_east_west_order(was_lower, side) {
            let idx = port_id(&port);
            if idx < self.port_positions.len() {
                self.port_positions[idx] = self.position_of(&port) - lower_shift;
            }
        }
    }

    fn connected_in_layer_ports_sorted_by_position(
        &self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
        side: PortSide,
    ) -> Vec<LPortRef> {
        let mut ports: Vec<LPortRef> = Vec::new();
        let mut edge_buf = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        for node in [upper_node, lower_node] {
            for port in in_north_south_east_west_order(node, side) {
                collect_connected_edges(&port, &mut edge_buf);
                for edge in &edge_buf {
                    if edge
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.is_self_loop())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    if seen.insert(port_ptr_id(&port)) {
                        ports.push(port.clone());
                    }
                    if is_in_layer(edge) {
                        let other = other_end_of(edge, &port);
                        if seen.insert(port_ptr_id(&other)) {
                            ports.push(other);
                        }
                    }
                }
            }
        }
        ports.sort_by_key(|port| self.position_of(port));
        ports
    }

    fn connected_ports_sorted_by_position(
        &self,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
    ) -> Vec<LPortRef> {
        let mut ports: Vec<LPortRef> = Vec::new();
        let mut edge_buf = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        for port in [upper_port, lower_port] {
            if seen.insert(port_ptr_id(port)) {
                ports.push(port.clone());
            }
            collect_connected_edges(port, &mut edge_buf);
            for edge in &edge_buf {
                if is_port_self_loop(edge) {
                    continue;
                }
                let other = other_end_of(edge, port);
                if seen.insert(port_ptr_id(&other)) {
                    ports.push(other);
                }
            }
        }
        ports.sort_by_key(|port| self.position_of(port));
        ports
    }

    fn count_crossings_on_ports(&mut self, ports: &[LPortRef]) -> i32 {
        let mut crossings = 0;
        let mut edge_buf = Vec::new();
        for port in ports {
            let current_position = self.position_of(port);
            self.index_tree.remove_all(current_position as usize);
            collect_connected_edges(port, &mut edge_buf);
            for edge in &edge_buf {
                let end_position = self.position_of(&other_end_of(edge, port));
                if end_position > current_position {
                    crossings += self.index_tree.rank(end_position as usize);
                    self.ends.push_back(end_position);
                }
            }
            while let Some(end_pos) = self.ends.pop_back() {
                self.index_tree.add(end_pos as usize);
            }
        }
        crossings
    }

    fn count_in_layer_crossings_on_ports(&mut self, ports: &[LPortRef]) -> i32 {
        let mut crossings = 0;
        let mut edge_buf = Vec::new();
        for port in ports {
            let current_position = self.position_of(port);
            self.index_tree.remove_all(current_position as usize);
            let mut num_between_layer_edges = 0;
            collect_connected_edges(port, &mut edge_buf);
            for edge in &edge_buf {
                if is_in_layer(edge) {
                    let end_position = self.position_of(&other_end_of(edge, port));
                    if end_position > current_position {
                        crossings += self.index_tree.rank(end_position as usize);
                        self.ends.push_back(end_position);
                    }
                } else {
                    num_between_layer_edges += 1;
                }
            }
            crossings += self.index_tree.size() * num_between_layer_edges;
            while let Some(end_pos) = self.ends.pop_back() {
                self.index_tree.add(end_pos as usize);
            }
        }
        crossings
    }

    fn count_north_south_crossings_on_ports(&mut self, ports: &[LPortRef]) -> i32 {
        let mut crossings = 0;
        let mut targets_and_degrees: Vec<(LPortRef, i32)> = Vec::new();

        for port in ports {
            self.index_tree.remove_all(self.position_of(port) as usize);
            targets_and_degrees.clear();

            let node_type = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
                .and_then(|node| node.lock().ok().map(|node_guard| node_guard.node_type()))
                .unwrap_or(NodeType::Normal);

            match node_type {
                NodeType::Normal => {
                    let dummy = port.lock().ok().and_then(|mut port_guard| {
                        port_guard.get_property(InternalProperties::PORT_DUMMY)
                    });
                    if let Some(dummy) = dummy {
                        let dummy_ports = dummy
                            .lock()
                            .ok()
                            .map(|node_guard| node_guard.ports().clone())
                            .unwrap_or_default();
                        for dummy_port in dummy_ports {
                            let degree = dummy_port
                                .lock()
                                .ok()
                                .map(|port_guard| port_guard.degree() as i32)
                                .unwrap_or(0);
                            targets_and_degrees.push((dummy_port, degree));
                        }
                    }
                }
                NodeType::LongEdge => {
                    let other_port = port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node())
                        .and_then(|node| {
                            node.lock()
                                .ok()
                                .map(|node_guard| node_guard.ports().clone())
                        })
                        .and_then(|ports| ports.into_iter().find(|p| !Arc::ptr_eq(p, port)));
                    if let Some(other_port) = other_port {
                        let degree = other_port
                            .lock()
                            .ok()
                            .map(|port_guard| port_guard.degree() as i32)
                            .unwrap_or(0);
                        targets_and_degrees.push((other_port, degree));
                    }
                }
                NodeType::NorthSouthPort => {
                    let origin_port = port
                        .lock()
                        .ok()
                        .and_then(|mut port_guard| {
                            port_guard.get_property(InternalProperties::ORIGIN)
                        })
                        .and_then(|origin| match origin {
                            Origin::LPort(port) => Some(port),
                            _ => None,
                        });
                    if let Some(origin_port) = origin_port {
                        let degree = port
                            .lock()
                            .ok()
                            .map(|port_guard| port_guard.degree() as i32)
                            .unwrap_or(0);
                        targets_and_degrees.push((origin_port, degree));
                    }
                }
                _ => {}
            }

            for (target_port, degree) in &targets_and_degrees {
                let end_position = self.position_of(target_port);
                if end_position > self.position_of(port) {
                    crossings += self.index_tree.rank(end_position as usize) * *degree;
                    self.ends.push_back(end_position);
                }
            }

            while let Some(end_pos) = self.ends.pop_back() {
                self.index_tree.add(end_pos as usize);
            }
        }

        crossings
    }

    fn init_positions(
        &mut self,
        nodes: &[LNodeRef],
        ports: &mut Vec<LPortRef>,
        side: PortSide,
        top_down: bool,
        get_cardinalities: bool,
    ) {
        if nodes.is_empty() {
            return;
        }
        let mut num_ports = ports.len() as i32;
        if get_cardinalities {
            self.node_cardinalities = vec![0; nodes.len()];
        }
        let mut i = if top_down {
            0
        } else {
            nodes.len() as isize - 1
        };
        while if top_down {
            i < nodes.len() as isize
        } else {
            i >= 0
        } {
            let node = nodes[i as usize].clone();
            let node_ports = self.get_ports(&node, side, top_down);
            if get_cardinalities {
                self.node_cardinalities[node_id(&node)] = node_ports.len() as i32;
            }
            for port in &node_ports {
                let pid = port_id(port);
                if pid >= self.port_positions.len() {
                    self.port_positions.resize(pid + 1, 0);
                }
                self.port_positions[pid] = num_ports;
                num_ports += 1;
            }
            ports.extend(node_ports);
            if top_down {
                i += 1;
            } else {
                i -= 1;
            }
        }
    }

    fn init_port_positions_counter_clockwise(
        &mut self,
        left_layer: &[LNodeRef],
        right_layer: &[LNodeRef],
    ) -> Vec<LPortRef> {
        let mut ports = Vec::new();
        self.init_positions(left_layer, &mut ports, PortSide::East, true, false);
        self.init_positions(right_layer, &mut ports, PortSide::West, false, false);
        ports
    }

    fn init_positions_for_north_south_counting(&mut self, nodes: &[LNodeRef]) -> Vec<LPortRef> {
        const INDEXING_SIDE: PortSide = PortSide::West;
        const STACK_SIDE: PortSide = PortSide::East;

        let mut ports: Vec<LPortRef> = Vec::new();
        let mut stack: Vec<LNodeRef> = Vec::new();
        let mut last_layout_unit: Option<LNodeRef> = None;
        let mut index: i32 = 0;

        for current in nodes {
            if is_layout_unit_changed(last_layout_unit.as_ref(), current) {
                index = empty_stack(
                    &mut stack,
                    &mut ports,
                    STACK_SIDE,
                    index,
                    &mut self.port_positions,
                );
            }
            if node_has_property(current, InternalProperties::IN_LAYER_LAYOUT_UNIT) {
                last_layout_unit = current.lock().ok().and_then(|mut node_guard| {
                    node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
                });
            }

            let node_type = current
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type())
                .unwrap_or(NodeType::Normal);
            match node_type {
                NodeType::Normal => {
                    for port in get_north_south_ports_with_incident_edges(current, PortSide::North)
                    {
                        set_port_position(&mut self.port_positions, &port, index);
                        index += 1;
                        ports.push(port);
                    }

                    index = empty_stack(
                        &mut stack,
                        &mut ports,
                        STACK_SIDE,
                        index,
                        &mut self.port_positions,
                    );

                    for port in get_north_south_ports_with_incident_edges(current, PortSide::South)
                    {
                        set_port_position(&mut self.port_positions, &port, index);
                        index += 1;
                        ports.push(port);
                    }
                }
                NodeType::NorthSouthPort => {
                    let west_ports = current
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.port_side_view(INDEXING_SIDE))
                        .unwrap_or_default();
                    if let Some(port) = west_ports.first() {
                        set_port_position(&mut self.port_positions, port, index);
                        index += 1;
                        ports.push(port.clone());
                    }
                    let east_ports = current
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.port_side_view(STACK_SIDE))
                        .unwrap_or_default();
                    if !east_ports.is_empty() {
                        stack.push(current.clone());
                    }
                }
                NodeType::LongEdge => {
                    for port in current
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.port_side_view(PortSide::West))
                        .unwrap_or_default()
                    {
                        set_port_position(&mut self.port_positions, &port, index);
                        index += 1;
                        ports.push(port);
                    }
                    let east_ports = current
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.port_side_view(PortSide::East))
                        .unwrap_or_default();
                    if !east_ports.is_empty() {
                        stack.push(current.clone());
                    }
                }
                _ => {}
            }
        }

        empty_stack(
            &mut stack,
            &mut ports,
            STACK_SIDE,
            index,
            &mut self.port_positions,
        );

        ports
    }

    fn get_ports(&self, node: &LNodeRef, side: PortSide, top_down: bool) -> Vec<LPortRef> {
        let ports = node
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(side))
            .unwrap_or_default();
        if side == PortSide::East {
            if top_down {
                ports
            } else {
                ports.into_iter().rev().collect()
            }
        } else if top_down {
            ports.into_iter().rev().collect()
        } else {
            ports
        }
    }

    fn position_of(&self, port: &LPortRef) -> i32 {
        let pid = port_id(port);
        *self.port_positions.get(pid).unwrap_or(&0)
    }
}

fn port_id(port: &LPortRef) -> usize {
    port.lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn collect_connected_edges(port: &LPortRef, out: &mut Vec<LEdgeRef>) {
    out.clear();
    if let Ok(port_guard) = port.lock() {
        out.extend(port_guard.incoming_edges().iter().cloned());
        out.extend(port_guard.outgoing_edges().iter().cloned());
    }
}

fn is_in_layer(edge: &LEdgeRef) -> bool {
    let (source_layer, target_layer) = edge
        .lock()
        .ok()
        .map(|edge_guard| {
            let source_layer = edge_guard
                .source()
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
            let target_layer = edge_guard
                .target()
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
            (source_layer, target_layer)
        })
        .unwrap_or((None, None));
    if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
        Arc::ptr_eq(&source_layer, &target_layer)
    } else {
        if *TRACE_CROSSINGS_BREAKDOWN {
            eprintln!("rust-crossings: is_in_layer missing layer endpoint");
        }
        false
    }
}

fn other_end_of(edge: &LEdgeRef, from_port: &LPortRef) -> LPortRef {
    let (source, target) = edge
        .lock()
        .ok()
        .map(|edge_guard| (edge_guard.source(), edge_guard.target()))
        .unwrap_or((None, None));
    match (source, target) {
        (Some(source), Some(target)) => {
            if Arc::ptr_eq(&source, from_port) {
                target
            } else {
                source
            }
        }
        _ => panic!("edge endpoint missing"),
    }
}

fn is_port_self_loop(edge: &LEdgeRef) -> bool {
    edge.lock()
        .ok()
        .map(|edge_guard| {
            let source = edge_guard.source();
            let target = edge_guard.target();
            match (source, target) {
                (Some(source), Some(target)) => Arc::ptr_eq(&source, &target),
                _ => false,
            }
        })
        .unwrap_or(false)
}

fn port_ptr_id(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn node_has_property(
    node: &LNodeRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<LNodeRef>,
) -> bool {
    node.lock()
        .ok()
        .map(|mut node_guard| {
            node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(property)
        })
        .unwrap_or(false)
}

fn is_layout_unit_changed(last_unit: Option<&LNodeRef>, node: &LNodeRef) -> bool {
    let Some(last_unit) = last_unit else {
        return false;
    };
    if Arc::ptr_eq(last_unit, node) {
        return false;
    }
    if !node_has_property(node, InternalProperties::IN_LAYER_LAYOUT_UNIT) {
        return false;
    }
    let unit = node.lock().ok().and_then(|mut node_guard| {
        node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
    });
    match unit {
        Some(unit) => !Arc::ptr_eq(&unit, last_unit),
        None => false,
    }
}

fn get_north_south_ports_with_incident_edges(node: &LNodeRef, side: PortSide) -> Vec<LPortRef> {
    node.lock()
        .ok()
        .map(|mut node_guard| {
            node_guard
                .port_side_view(side)
                .into_iter()
                .filter(|port| port_has_property(port, InternalProperties::PORT_DUMMY))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn port_has_property(
    port: &LPortRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<LNodeRef>,
) -> bool {
    port.lock()
        .ok()
        .map(|mut port_guard| {
            port_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(property)
        })
        .unwrap_or(false)
}

fn empty_stack(
    stack: &mut Vec<LNodeRef>,
    ports: &mut Vec<LPortRef>,
    side: PortSide,
    mut index: i32,
    port_positions: &mut Vec<i32>,
) -> i32 {
    while let Some(dummy) = stack.pop() {
        let dummy_ports = dummy
            .lock()
            .ok()
            .map(|mut node_guard| node_guard.port_side_view(side))
            .unwrap_or_default();
        if let Some(port) = dummy_ports.first() {
            set_port_position(port_positions, port, index);
            index += 1;
            ports.push(port.clone());
        }
    }
    index
}

fn set_port_position(port_positions: &mut Vec<i32>, port: &LPortRef, position: i32) {
    let pid = port_id(port);
    if pid >= port_positions.len() {
        port_positions.resize(pid + 1, 0);
    }
    port_positions[pid] = position;
}
