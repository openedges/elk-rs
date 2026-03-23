use std::collections::VecDeque;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, Origin};
use crate::org::eclipse::elk::alg::layered::p3order::counting::{
    in_north_south_east_west_order, BinaryIndexedTree,
};
use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;

pub struct CrossingsCounter {
    port_positions: Vec<i32>,
    index_tree: BinaryIndexedTree,
    ends: VecDeque<i32>,
    node_cardinalities: Vec<i32>,
    snapshot: Option<Arc<CrossMinSnapshot>>,
    /// Reusable scratch buffer for port deduplication (replaces per-call BTreeSet)
    scratch_seen: Vec<usize>,
    /// Reusable scratch buffer for edge collection
    scratch_edge_buf: Vec<LEdgeRef>,
}

impl CrossingsCounter {
    pub fn new(port_positions: Vec<i32>) -> Self {
        CrossingsCounter {
            port_positions,
            index_tree: BinaryIndexedTree::new(0),
            ends: VecDeque::new(),
            node_cardinalities: Vec::new(),
            snapshot: None,
            scratch_seen: Vec::new(),
            scratch_edge_buf: Vec::new(),
        }
    }

    pub fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.snapshot = Some(snapshot);
    }

    #[inline]
    fn port_id_of(&self, port: &LPortRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.port_id(port) as usize
        } else {
            port_id(port)
        }
    }

    #[inline]
    fn node_id_of(&self, node: &LNodeRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.node_id(node) as usize
        } else {
            node_id(node)
        }
    }

    pub fn count_crossings_between_layers(
        &mut self,
        left_layer: &[LNodeRef],
        right_layer: &[LNodeRef],
    ) -> i32 {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            let port_ids =
                self.init_port_positions_counter_clockwise_snap(snap, left_layer, right_layer);
            self.index_tree.reset(port_ids.len());
            self.count_crossings_on_ports_snap(snap, &port_ids)
        } else {
            let ports = self.init_port_positions_counter_clockwise(left_layer, right_layer);
            self.index_tree.reset(ports.len());
            self.count_crossings_on_ports(&ports)
        }
    }

    pub fn count_in_layer_crossings_on_side(&mut self, nodes: &[LNodeRef], side: PortSide) -> i32 {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            let port_ids = self.init_port_positions_for_in_layer_crossings_snap(snap, nodes, side);
            self.count_in_layer_crossings_on_ports_snap(snap, &port_ids)
        } else {
            let ports = self.init_port_positions_for_in_layer_crossings(nodes, side);
            self.count_in_layer_crossings_on_ports(&ports)
        }
    }

    pub fn count_north_south_port_crossings_in_layer(&mut self, layer: &[LNodeRef]) -> i32 {
        let ports = self.init_positions_for_north_south_counting(layer);
        self.index_tree.reset(ports.len());
        self.count_north_south_crossings_on_ports(&ports)
    }

    pub fn count_crossings_between_ports_in_both_orders(
        &mut self,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
    ) -> Pair<i32, i32> {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            let mut ports =
                self.connected_ports_sorted_by_position_snap(snap, upper_port, lower_port);
            let upper_lower_crossings = self.count_crossings_on_ports_snap(snap, &ports);
            self.index_tree.clear();
            let up_id = snap.port_id(upper_port) as usize;
            let lo_id = snap.port_id(lower_port) as usize;
            if up_id < self.port_positions.len() && lo_id < self.port_positions.len() {
                self.port_positions.swap(up_id, lo_id);
            }
            ports.sort_by_key(|&pid| *self.port_positions.get(pid as usize).unwrap_or(&0));
            let lower_upper_crossings = self.count_crossings_on_ports_snap(snap, &ports);
            self.index_tree.clear();
            if up_id < self.port_positions.len() && lo_id < self.port_positions.len() {
                self.port_positions.swap(up_id, lo_id);
            }
            Pair::of(upper_lower_crossings, lower_upper_crossings)
        } else {
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
    }

    pub fn count_in_layer_crossings_between_nodes_in_both_orders(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
        side: PortSide,
    ) -> Pair<i32, i32> {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            let mut ports = self.connected_in_layer_ports_sorted_by_position_snap(
                snap, upper_node, lower_node, side,
            );
            let upper_lower_crossings = self.count_in_layer_crossings_on_ports_snap(snap, &ports);
            self.switch_nodes(upper_node, lower_node, side);
            self.index_tree.clear();
            ports.sort_by_key(|&pid| *self.port_positions.get(pid as usize).unwrap_or(&0));
            let lower_upper_crossings = self.count_in_layer_crossings_on_ports_snap(snap, &ports);
            self.switch_nodes(lower_node, upper_node, side);
            self.index_tree.clear();
            Pair::of(upper_lower_crossings, lower_upper_crossings)
        } else {
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
    }

    pub fn init_for_counting_between(&mut self, left_layer: &[LNodeRef], right_layer: &[LNodeRef]) {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            let port_ids =
                self.init_port_positions_counter_clockwise_snap(snap, left_layer, right_layer);
            self.index_tree.reset(port_ids.len());
        } else {
            let ports = self.init_port_positions_counter_clockwise(left_layer, right_layer);
            self.index_tree.reset(ports.len());
        }
    }

    pub fn init_port_positions_for_in_layer_crossings(
        &mut self,
        nodes: &[LNodeRef],
        side: PortSide,
    ) -> Vec<LPortRef> {
        let mut ports = Vec::new();
        self.init_positions(nodes, &mut ports, side, true, true);
        self.index_tree.reset(ports.len());
        ports
    }

    pub fn switch_ports(&mut self, top_port: &LPortRef, bottom_port: &LPortRef) {
        let top_index = self.port_id_of(top_port);
        let bottom_index = self.port_id_of(bottom_port);
        if top_index >= self.port_positions.len() || bottom_index >= self.port_positions.len() {
            return;
        }
        self.port_positions.swap(top_index, bottom_index);
    }

    pub fn switch_nodes(&mut self, was_upper: &LNodeRef, was_lower: &LNodeRef, side: PortSide) {
        let snap = self.snapshot.clone();
        if let Some(ref snap) = snap {
            self.switch_nodes_snap(snap, was_upper, was_lower, side);
        } else {
            self.switch_nodes_arc(was_upper, was_lower, side);
        }
    }

    fn switch_nodes_arc(&mut self, was_upper: &LNodeRef, was_lower: &LNodeRef, side: PortSide) {
        let upper_id = self.node_id_of(was_upper);
        let lower_id = self.node_id_of(was_lower);
        let upper_shift = *self.node_cardinalities.get(lower_id).unwrap_or(&0);
        let lower_shift = *self.node_cardinalities.get(upper_id).unwrap_or(&0);

        for port in in_north_south_east_west_order(was_upper, side) {
            let idx = self.port_id_of(&port);
            if idx < self.port_positions.len() {
                self.port_positions[idx] = self.position_of(&port) + upper_shift;
            }
        }

        for port in in_north_south_east_west_order(was_lower, side) {
            let idx = self.port_id_of(&port);
            if idx < self.port_positions.len() {
                self.port_positions[idx] = self.position_of(&port) - lower_shift;
            }
        }
    }

    /// Snapshot path: uses CSR port-by-side lists, no node lock needed.
    fn switch_nodes_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        was_upper: &LNodeRef,
        was_lower: &LNodeRef,
        side: PortSide,
    ) {
        let upper_id = snap.node_id(was_upper) as usize;
        let lower_id = snap.node_id(was_lower) as usize;
        let upper_shift = *self.node_cardinalities.get(lower_id).unwrap_or(&0);
        let lower_shift = *self.node_cardinalities.get(upper_id).unwrap_or(&0);

        for pid in self.nsew_ports_snap(snap, was_upper, side) {
            let pid_usize = pid as usize;
            if pid_usize < self.port_positions.len() {
                self.port_positions[pid_usize] += upper_shift;
            }
        }

        for pid in self.nsew_ports_snap(snap, was_lower, side) {
            let pid_usize = pid as usize;
            if pid_usize < self.port_positions.len() {
                self.port_positions[pid_usize] -= lower_shift;
            }
        }
    }

    fn connected_in_layer_ports_sorted_by_position(
        &mut self,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
        side: PortSide,
    ) -> Vec<LPortRef> {
        let mut ports: Vec<LPortRef> = Vec::new();
        self.scratch_seen.clear();
        self.scratch_edge_buf.clear();
        for node in [upper_node, lower_node] {
            for port in in_north_south_east_west_order(node, side) {
                collect_connected_edges(&port, &mut self.scratch_edge_buf);
                for edge in &self.scratch_edge_buf {
                    if edge
                        .lock().is_self_loop()
                    {
                        continue;
                    }
                    let pid = port_ptr_id(&port);
                    if !self.scratch_seen.contains(&pid) {
                        self.scratch_seen.push(pid);
                        ports.push(port.clone());
                    }
                    if is_in_layer(edge) {
                        let other = other_end_of(edge, &port);
                        let oid = port_ptr_id(&other);
                        if !self.scratch_seen.contains(&oid) {
                            self.scratch_seen.push(oid);
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
        &mut self,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
    ) -> Vec<LPortRef> {
        let mut ports: Vec<LPortRef> = Vec::new();
        self.scratch_seen.clear();
        self.scratch_edge_buf.clear();
        for port in [upper_port, lower_port] {
            let pid = port_ptr_id(port);
            if !self.scratch_seen.contains(&pid) {
                self.scratch_seen.push(pid);
                ports.push(port.clone());
            }
            collect_connected_edges(port, &mut self.scratch_edge_buf);
            for edge in &self.scratch_edge_buf {
                if is_port_self_loop(edge) {
                    continue;
                }
                let other = other_end_of(edge, port);
                let oid = port_ptr_id(&other);
                if !self.scratch_seen.contains(&oid) {
                    self.scratch_seen.push(oid);
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
        let snap = self.snapshot.clone();

        for port in ports {
            self.index_tree.remove_all(self.position_of(port) as usize);
            targets_and_degrees.clear();

            let node_type = if let Some(ref snap) = snap {
                let pid = snap.port_id(port);
                snap.node_type_of(snap.port_owner_flat(pid))
            } else {
                port.lock().node()
                    .map(|node| node.lock().node_type())
                    .unwrap_or(NodeType::Normal)
            };

            match node_type {
                NodeType::Normal => {
                    // PORT_DUMMY property still needs one lock
                    let dummy = {
                        let port_guard = port.lock();
                        port_guard.get_property(InternalProperties::PORT_DUMMY)
                    };
                    if let Some(dummy) = dummy {
                        if let Some(ref snap) = snap {
                            // Snapshot path: use CSR for ports + degree
                            let dummy_flat = snap.node_flat_index(&dummy);
                            for &dpid in snap.node_ports(dummy_flat) {
                                let degree = (snap.port_predecessors(dpid).len()
                                    + snap.port_successors(dpid).len())
                                    as i32;
                                if let Some(dp_ref) = snap.port_ref_opt(dpid) {
                                    targets_and_degrees.push((dp_ref.clone(), degree));
                                }
                            }
                        } else {
                            let dummy_ports = dummy
                                .lock().ports().clone();
                            for dummy_port in dummy_ports {
                                let degree = {
                                    let port_guard = dummy_port.lock();
                                    port_guard.degree() as i32
                                };
                                targets_and_degrees.push((dummy_port, degree));
                            }
                        }
                    }
                }
                NodeType::LongEdge => {
                    if let Some(ref snap) = snap {
                        // Snapshot path: find other port on same node via CSR
                        let pid = snap.port_id(port);
                        let flat = snap.port_owner_flat(pid);
                        let other = snap
                            .node_ports(flat)
                            .iter()
                            .find(|&&p| p != pid)
                            .copied();
                        if let Some(other_pid) = other {
                            let degree = (snap.port_predecessors(other_pid).len()
                                + snap.port_successors(other_pid).len())
                                as i32;
                            if let Some(op_ref) = snap.port_ref_opt(other_pid) {
                                targets_and_degrees.push((op_ref.clone(), degree));
                            }
                        }
                    } else {
                        let other_port = port
                            .lock().node()
                            .and_then(|node| {
                                let node_guard = node.lock();
                                let ports = node_guard.ports().clone();
                                ports.into_iter().find(|p| !Arc::ptr_eq(p, port))
                            });
                        if let Some(other_port) = other_port {
                            let degree = {
                                let port_guard = other_port.lock();
                                port_guard.degree() as i32
                            };
                            targets_and_degrees.push((other_port, degree));
                        }
                    }
                }
                NodeType::NorthSouthPort => {
                    // ORIGIN property still needs one lock
                    let origin_port = {
                        let port_guard = port.lock();
                        port_guard.get_property(InternalProperties::ORIGIN)
                            .and_then(|origin| match origin {
                                Origin::LPort(port) => Some(port),
                                _ => None,
                            })
                    };
                    if let Some(origin_port) = origin_port {
                        let degree = if let Some(ref snap) = snap {
                            let pid = snap.port_id(port);
                            (snap.port_predecessors(pid).len()
                                + snap.port_successors(pid).len())
                                as i32
                        } else {
                            let port_guard = port.lock();
                            port_guard.degree() as i32
                        };
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
            self.node_cardinalities.clear();
            self.node_cardinalities.resize(nodes.len(), 0);
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
                let nid = self.node_id_of(&node);
                self.node_cardinalities[nid] = node_ports.len() as i32;
            }
            for port in &node_ports {
                let pid = self.port_id_of(port);
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

    // ── Snapshot-specialized init methods (direct Vec<u32>, no Arc clones) ───

    fn init_port_positions_counter_clockwise_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        left_layer: &[LNodeRef],
        right_layer: &[LNodeRef],
    ) -> Vec<u32> {
        let mut port_ids = Vec::new();
        self.init_positions_snap(snap, left_layer, &mut port_ids, PortSide::East, true, false);
        self.init_positions_snap(snap, right_layer, &mut port_ids, PortSide::West, false, false);
        port_ids
    }

    fn init_port_positions_for_in_layer_crossings_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        nodes: &[LNodeRef],
        side: PortSide,
    ) -> Vec<u32> {
        let mut port_ids = Vec::new();
        self.init_positions_snap(snap, nodes, &mut port_ids, side, true, true);
        self.index_tree.reset(port_ids.len());
        port_ids
    }

    fn init_positions_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        nodes: &[LNodeRef],
        port_ids: &mut Vec<u32>,
        side: PortSide,
        top_down: bool,
        get_cardinalities: bool,
    ) {
        if nodes.is_empty() {
            return;
        }
        let mut num_ports = port_ids.len() as i32;
        if get_cardinalities {
            self.node_cardinalities.clear();
            self.node_cardinalities.resize(nodes.len(), 0);
        }
        let indices: Box<dyn Iterator<Item = usize>> = if top_down {
            Box::new(0..nodes.len())
        } else {
            Box::new((0..nodes.len()).rev())
        };
        let mut filtered: Vec<u32> = Vec::new();
        for i in indices {
            let node = &nodes[i];
            // Lock node once to get port IDs in current order, filtered by side.
            // Reuse buffer — no per-node allocation.
            filtered.clear();
            {
                let node_guard = node.lock();
                for p in node_guard.ports() {
                    let pid = snap.port_id(p);
                    if snap.port_side_of(pid) == side {
                        filtered.push(pid);
                    }
                }
                // Apply reversal matching get_ports logic
                match side {
                    PortSide::East => {
                        if !top_down {
                            filtered.reverse();
                        }
                    }
                    _ => {
                        if top_down {
                            filtered.reverse();
                        }
                    }
                }
            }

            if get_cardinalities {
                let nid = snap.node_id(node) as usize;
                if nid < self.node_cardinalities.len() {
                    self.node_cardinalities[nid] = filtered.len() as i32;
                }
            }

            for &pid in &filtered {
                let pid_usize = pid as usize;
                if pid_usize >= self.port_positions.len() {
                    self.port_positions.resize(pid_usize + 1, 0);
                }
                self.port_positions[pid_usize] = num_ports;
                num_ports += 1;
            }
            port_ids.extend_from_slice(&filtered);
        }
    }

    fn init_positions_for_north_south_counting(&mut self, nodes: &[LNodeRef]) -> Vec<LPortRef> {
        const INDEXING_SIDE: PortSide = PortSide::West;
        const STACK_SIDE: PortSide = PortSide::East;

        let mut ports: Vec<LPortRef> = Vec::new();
        let mut stack: Vec<LNodeRef> = Vec::new();
        let mut last_layout_unit: Option<LNodeRef> = None;
        let mut index: i32 = 0;

        for current in nodes {
            // ── Single lock per node: extract node_type, layout_unit, and side ports ──
            // Replaces 5-8 separate locks per node with 1.
            let (node_type_val, layout_unit, north_ports, south_ports, west_ports, east_nonempty) = {
                let mut ng = current.lock();
                let nt = if let Some(ref snap) = self.snapshot {
                    snap.node_type_of(snap.node_flat_index(current))
                } else {
                    ng.node_type()
                };
                let ilu = ng.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT);
                match nt {
                    NodeType::Normal => {
                        let n = ng.port_side_view(PortSide::North);
                        let s = ng.port_side_view(PortSide::South);
                        (nt, ilu, n, s, Vec::new(), false)
                    }
                    NodeType::NorthSouthPort => {
                        let w = ng.port_side_view(INDEXING_SIDE);
                        let e_ne = !ng.port_side_view(STACK_SIDE).is_empty();
                        (nt, ilu, Vec::new(), Vec::new(), w, e_ne)
                    }
                    NodeType::LongEdge => {
                        let w = ng.port_side_view(PortSide::West);
                        let e_ne = !ng.port_side_view(PortSide::East).is_empty();
                        (nt, ilu, Vec::new(), Vec::new(), w, e_ne)
                    }
                    _ => (nt, ilu, Vec::new(), Vec::new(), Vec::new(), false),
                }
            };

            // Layout unit change check (zero locks)
            let ilu_changed = match &last_layout_unit {
                None => false,
                Some(last) => {
                    if Arc::ptr_eq(last, current) {
                        false
                    } else {
                        layout_unit.as_ref().map_or(false, |u| !Arc::ptr_eq(u, last))
                    }
                }
            };
            if ilu_changed {
                index = empty_stack(
                    &mut stack,
                    &mut ports,
                    STACK_SIDE,
                    index,
                    &mut self.port_positions,
                    &self.snapshot,
                );
            }
            if layout_unit.is_some() {
                last_layout_unit = layout_unit;
            }

            match node_type_val {
                NodeType::Normal => {
                    for port in north_ports.into_iter()
                        .filter(|p| port_has_property(p, InternalProperties::PORT_DUMMY))
                    {
                        let pid = self.port_id_of(&port);
                        set_port_position(&mut self.port_positions, pid, index);
                        index += 1;
                        ports.push(port);
                    }

                    index = empty_stack(
                        &mut stack,
                        &mut ports,
                        STACK_SIDE,
                        index,
                        &mut self.port_positions,
                        &self.snapshot,
                    );

                    for port in south_ports.into_iter()
                        .filter(|p| port_has_property(p, InternalProperties::PORT_DUMMY))
                    {
                        let pid = self.port_id_of(&port);
                        set_port_position(&mut self.port_positions, pid, index);
                        index += 1;
                        ports.push(port);
                    }
                }
                NodeType::NorthSouthPort => {
                    if let Some(port) = west_ports.first() {
                        let pid = self.port_id_of(port);
                        set_port_position(&mut self.port_positions, pid, index);
                        index += 1;
                        ports.push(port.clone());
                    }
                    if east_nonempty {
                        stack.push(current.clone());
                    }
                }
                NodeType::LongEdge => {
                    for port in &west_ports {
                        let pid = self.port_id_of(port);
                        set_port_position(&mut self.port_positions, pid, index);
                        index += 1;
                        ports.push(port.clone());
                    }
                    if east_nonempty {
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
            &self.snapshot,
        );

        ports
    }

    fn get_ports(&self, node: &LNodeRef, side: PortSide, top_down: bool) -> Vec<LPortRef> {
        if let Some(ref snap) = self.snapshot {
            // Snapshot path: get port IDs filtered by side (no Arc clone of ports list),
            // then convert to LPortRef via reverse map. Still needs one node lock to get
            // current port order (port distributor may have reordered).
            let mut port_refs: Vec<LPortRef> = {
                let node_guard = node.lock();
                node_guard
                    .ports()
                    .iter()
                    .filter_map(|p| {
                        let pid = snap.port_id(p);
                        if snap.port_side_of(pid) == side {
                            snap.port_ref_opt(pid).cloned()
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            };
            let need_reverse = if side == PortSide::East { !top_down } else { top_down };
            if need_reverse {
                port_refs.reverse();
            }
            port_refs
        } else {
            let mut ports = {
                let mut node_guard = node.lock();
                node_guard.port_side_view(side)
            };
            let need_reverse = if side == PortSide::East { !top_down } else { top_down };
            if need_reverse {
                ports.reverse();
            }
            ports
        }
    }

    // ── Snapshot-based methods (lock-free CSR adjacency) ────────────────

    fn nsew_ports_snap(
        &self,
        snap: &CrossMinSnapshot,
        node: &LNodeRef,
        side: PortSide,
    ) -> Vec<u32> {
        let flat = snap.node_flat_index(node);
        let all_ports = snap.node_ports(flat);
        let mut ports: Vec<u32> = all_ports
            .iter()
            .copied()
            .filter(|&pid| snap.port_side_of(pid) == side)
            .collect();
        match side {
            PortSide::South | PortSide::West => ports.reverse(),
            _ => {}
        }
        ports
    }

    fn count_crossings_on_ports_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        ports: &[u32],
    ) -> i32 {
        let mut crossings = 0;
        for &pid in ports {
            let current_position = *self.port_positions.get(pid as usize).unwrap_or(&0);
            self.index_tree.remove_all(current_position as usize);
            for &other_pid in snap.port_predecessors(pid) {
                let end_position = *self.port_positions.get(other_pid as usize).unwrap_or(&0);
                if end_position > current_position {
                    crossings += self.index_tree.rank(end_position as usize);
                    self.ends.push_back(end_position);
                }
            }
            for &other_pid in snap.port_successors(pid) {
                let end_position = *self.port_positions.get(other_pid as usize).unwrap_or(&0);
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

    fn count_in_layer_crossings_on_ports_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        ports: &[u32],
    ) -> i32 {
        let mut crossings = 0;
        for &pid in ports {
            let current_position = *self.port_positions.get(pid as usize).unwrap_or(&0);
            self.index_tree.remove_all(current_position as usize);
            let pid_layer = snap.port_owner_layer(pid);
            let mut num_between_layer_edges = 0;
            for &other_pid in snap.port_predecessors(pid) {
                if snap.port_owner_layer(other_pid) == pid_layer {
                    let end_position =
                        *self.port_positions.get(other_pid as usize).unwrap_or(&0);
                    if end_position > current_position {
                        crossings += self.index_tree.rank(end_position as usize);
                        self.ends.push_back(end_position);
                    }
                } else {
                    num_between_layer_edges += 1;
                }
            }
            for &other_pid in snap.port_successors(pid) {
                if snap.port_owner_layer(other_pid) == pid_layer {
                    let end_position =
                        *self.port_positions.get(other_pid as usize).unwrap_or(&0);
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

    fn connected_ports_sorted_by_position_snap(
        &self,
        snap: &CrossMinSnapshot,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
    ) -> Vec<u32> {
        let mut ports: Vec<u32> = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        for port in [upper_port, lower_port] {
            let pid = snap.port_id(port);
            if !seen.contains(&pid) {
                seen.push(pid);
                ports.push(pid);
            }
            for &other_pid in snap.port_predecessors(pid) {
                if other_pid == pid {
                    continue;
                }
                if !seen.contains(&other_pid) {
                    seen.push(other_pid);
                    ports.push(other_pid);
                }
            }
            for &other_pid in snap.port_successors(pid) {
                if other_pid == pid {
                    continue;
                }
                if !seen.contains(&other_pid) {
                    seen.push(other_pid);
                    ports.push(other_pid);
                }
            }
        }
        ports.sort_by_key(|&pid| *self.port_positions.get(pid as usize).unwrap_or(&0));
        ports
    }

    fn connected_in_layer_ports_sorted_by_position_snap(
        &self,
        snap: &CrossMinSnapshot,
        upper_node: &LNodeRef,
        lower_node: &LNodeRef,
        side: PortSide,
    ) -> Vec<u32> {
        let mut ports: Vec<u32> = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        for node in [upper_node, lower_node] {
            let node_ports = self.nsew_ports_snap(snap, node, side);
            for pid in node_ports {
                for &other_pid in snap
                    .port_predecessors(pid)
                    .iter()
                    .chain(snap.port_successors(pid))
                {
                    if snap.port_owner_flat(pid) == snap.port_owner_flat(other_pid) {
                        continue;
                    }
                    if !seen.contains(&pid) {
                        seen.push(pid);
                        ports.push(pid);
                    }
                    if snap.port_owner_layer(pid) == snap.port_owner_layer(other_pid)
                        && !seen.contains(&other_pid)
                    {
                        seen.push(other_pid);
                        ports.push(other_pid);
                    }
                }
            }
        }
        ports.sort_by_key(|&pid| *self.port_positions.get(pid as usize).unwrap_or(&0));
        ports
    }

    fn position_of(&self, port: &LPortRef) -> i32 {
        let pid = self.port_id_of(port);
        *self.port_positions.get(pid).unwrap_or(&0)
    }
}

fn port_id(port: &LPortRef) -> usize {
    let mut port_guard = port.lock();
    port_guard.shape().graph_element().id as usize
}

fn node_id(node: &LNodeRef) -> usize {
    let mut node_guard = node.lock();
    node_guard.shape().graph_element().id as usize
}

fn collect_connected_edges(port: &LPortRef, out: &mut Vec<LEdgeRef>) {
    out.clear();
    {
        let port_guard = port.lock();
        out.extend(port_guard.incoming_edges().iter().cloned());
        out.extend(port_guard.outgoing_edges().iter().cloned());
    }
}

fn is_in_layer(edge: &LEdgeRef) -> bool {
    let (source_layer, target_layer) = {
        let edge_guard = edge.lock();
        let source_layer = edge_guard
            .source()
            .and_then(|port| port.lock().node())
            .and_then(|node| node.lock().layer());
        let target_layer = edge_guard
            .target()
            .and_then(|port| port.lock().node())
            .and_then(|node| node.lock().layer());
        (source_layer, target_layer)
    };
    if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
        Arc::ptr_eq(&source_layer, &target_layer)
    } else {
        if ElkTrace::global().crossings_breakdown {
            eprintln!("rust-crossings: is_in_layer missing layer endpoint");
        }
        false
    }
}

fn other_end_of(edge: &LEdgeRef, from_port: &LPortRef) -> LPortRef {
    let edge_guard = edge.lock();
    let source = edge_guard.source();
    let target = edge_guard.target();
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
    let edge_guard = edge.lock();
    let source = edge_guard.source();
    let target = edge_guard.target();
    match (source, target) {
        (Some(source), Some(target)) => Arc::ptr_eq(&source, &target),
        _ => false,
    }
}

fn port_ptr_id(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn port_has_property(
    port: &LPortRef,
    property: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<LNodeRef>,
) -> bool {
    let mut port_guard = port.lock();
    port_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(property)
}

fn empty_stack(
    stack: &mut Vec<LNodeRef>,
    ports: &mut Vec<LPortRef>,
    side: PortSide,
    mut index: i32,
    port_positions: &mut Vec<i32>,
    snapshot: &Option<Arc<CrossMinSnapshot>>,
) -> i32 {
    while let Some(dummy) = stack.pop() {
        let dummy_ports = {
            let mut node_guard = dummy.lock();
            node_guard.port_side_view(side)
        };
        if let Some(port) = dummy_ports.first() {
            let pid = if let Some(ref snap) = snapshot {
                snap.port_id(port) as usize
            } else {
                port_id(port)
            };
            set_port_position(port_positions, pid, index);
            index += 1;
            ports.push(port.clone());
        }
    }
    index
}

fn set_port_position(port_positions: &mut Vec<i32>, pid: usize, position: i32) {
    if pid >= port_positions.len() {
        port_positions.resize(pid + 1, 0);
    }
    port_positions[pid] = position;
}
