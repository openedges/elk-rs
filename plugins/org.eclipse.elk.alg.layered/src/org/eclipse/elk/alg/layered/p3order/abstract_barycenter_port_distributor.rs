use std::cmp::Ordering;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

static TRACE_PORT_RANKS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_PORT_RANKS").is_some());
static TRACE_CROSSMIN_TIMING: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN_TIMING").is_some());

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, PortType,
};
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;

/// Truncate f64 to f32 precision, matching Java's float[] storage.
#[inline(always)]
fn f32t(v: f64) -> f64 {
    v as f32 as f64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortRankStrategy {
    NodeRelative,
    LayerTotal,
}

pub struct AbstractBarycenterPortDistributor {
    strategy: PortRankStrategy,
    port_ranks: Vec<f64>,
    min_barycenter: f64,
    max_barycenter: f64,
    node_positions: Vec<Vec<usize>>,
    port_barycenter: Vec<f64>,
    in_layer_ports: Vec<LPortRef>,
    n_ports: usize,
    // Reusable buffers for ports_by_side partitioning (avoids 3x Vec alloc per node per sweep)
    side_buf: Vec<LPortRef>,
    south_buf: Vec<LPortRef>,
    north_buf: Vec<LPortRef>,
}

impl AbstractBarycenterPortDistributor {
    pub fn new(num_layers: usize, strategy: PortRankStrategy) -> Self {
        AbstractBarycenterPortDistributor {
            strategy,
            port_ranks: Vec::new(),
            min_barycenter: 0.0,
            max_barycenter: 0.0,
            node_positions: vec![Vec::new(); num_layers],
            port_barycenter: Vec::new(),
            in_layer_ports: Vec::new(),
            n_ports: 0,
            side_buf: Vec::new(),
            south_buf: Vec::new(),
            north_buf: Vec::new(),
        }
    }

    pub fn port_ranks(&self) -> &Vec<f64> {
        &self.port_ranks
    }

    fn calculate_port_ranks_for_node(
        &mut self,
        node: &LNodeRef,
        rank_sum: f64,
        port_type: PortType,
    ) -> f64 {
        match self.strategy {
            PortRankStrategy::NodeRelative => {
                self.calculate_port_ranks_node_relative(node, rank_sum, port_type)
            }
            PortRankStrategy::LayerTotal => {
                self.calculate_port_ranks_layer_total(node, rank_sum, port_type)
            }
        }
    }

    fn calculate_port_ranks_node_relative(
        &mut self,
        node: &LNodeRef,
        rank_sum: f64,
        port_type: PortType,
    ) -> f64 {
        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        match port_type {
            PortType::Input => {
                let mut input_count = 0usize;
                let mut north_input_count = 0usize;
                for port in &ports {
                    let incoming = port
                        .lock()
                        .ok()
                        .map(|port_guard| !port_guard.incoming_edges().is_empty())
                        .unwrap_or(false);
                    if incoming {
                        input_count += 1;
                        let side = port
                            .lock()
                            .ok()
                            .map(|port_guard| port_guard.side())
                            .unwrap_or(PortSide::Undefined);
                        if side == PortSide::North {
                            north_input_count += 1;
                        }
                    }
                }
                if input_count == 0 {
                    return 1.0;
                }
                let incr = 1.0 / (input_count as f64 + 1.0);
                let mut north_pos = rank_sum + (north_input_count as f64) * incr;
                let mut rest_pos = rank_sum + 1.0 - incr;
                let input_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_type(PortType::Input))
                    .unwrap_or_default();
                for port in input_ports {
                    let pid = port_id(&port);
                    self.ensure_port_capacity(pid);
                    let side = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.side())
                        .unwrap_or(PortSide::Undefined);
                    if side == PortSide::North {
                        self.port_ranks[pid] = f32t(north_pos);
                        north_pos -= incr;
                    } else {
                        self.port_ranks[pid] = f32t(rest_pos);
                        rest_pos -= incr;
                    }
                }
                1.0
            }
            PortType::Output => {
                let mut output_count = 0usize;
                for port in &ports {
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| !port_guard.outgoing_edges().is_empty())
                        .unwrap_or(false);
                    if outgoing {
                        output_count += 1;
                    }
                }
                if output_count == 0 {
                    return 1.0;
                }
                let incr = 1.0 / (output_count as f64 + 1.0);
                let mut pos = rank_sum + incr;
                let output_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_type(PortType::Output))
                    .unwrap_or_default();
                for port in output_ports {
                    let pid = port_id(&port);
                    self.ensure_port_capacity(pid);
                    self.port_ranks[pid] = f32t(pos);
                    pos += incr;
                }
                1.0
            }
            _ => 1.0,
        }
    }

    fn calculate_port_ranks_layer_total(
        &mut self,
        node: &LNodeRef,
        rank_sum: f64,
        port_type: PortType,
    ) -> f64 {
        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        match port_type {
            PortType::Input => {
                let mut input_count = 0usize;
                let mut north_input_count = 0usize;
                for port in &ports {
                    let incoming = port
                        .lock()
                        .ok()
                        .map(|port_guard| !port_guard.incoming_edges().is_empty())
                        .unwrap_or(false);
                    if incoming {
                        input_count += 1;
                        let side = port
                            .lock()
                            .ok()
                            .map(|port_guard| port_guard.side())
                            .unwrap_or(PortSide::Undefined);
                        if side == PortSide::North {
                            north_input_count += 1;
                        }
                    }
                }
                if input_count == 0 {
                    return 0.0;
                }
                let mut north_pos = rank_sum + north_input_count as f64;
                let mut rest_pos = rank_sum + input_count as f64;
                let input_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_type(PortType::Input))
                    .unwrap_or_default();
                for port in input_ports {
                    let pid = port_id(&port);
                    self.ensure_port_capacity(pid);
                    let side = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.side())
                        .unwrap_or(PortSide::Undefined);
                    if side == PortSide::North {
                        self.port_ranks[pid] = f32t(north_pos);
                        north_pos -= 1.0;
                    } else {
                        self.port_ranks[pid] = f32t(rest_pos);
                        rest_pos -= 1.0;
                    }
                }
                input_count as f64
            }
            PortType::Output => {
                let output_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_type(PortType::Output))
                    .unwrap_or_default();
                let mut pos = 0.0;
                for port in output_ports {
                    pos += 1.0;
                    let pid = port_id(&port);
                    self.ensure_port_capacity(pid);
                    self.port_ranks[pid] = f32t(rank_sum + pos);
                }
                pos
            }
            _ => 0.0,
        }
    }

    fn distribute_ports(
        &mut self,
        node: &LNodeRef,
        side: PortSide,
        layer_index: usize,
        layer_size: usize,
    ) {
        let timing = *TRACE_CROSSMIN_TIMING;
        let (node_id, constraints, ports_snapshot) = if let Ok(mut node_guard) = node.lock() {
            (
                node_guard.shape().graph_element().id,
                node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined),
                node_guard.ports().clone(),
            )
        } else {
            (-1, PortConstraints::Undefined, Vec::new())
        };
        if timing {
            eprintln!(
                "crossmin: distribute_ports begin node={} side={:?} layer={} ports={} constraints={:?}",
                node_id,
                side,
                layer_index,
                ports_snapshot.len(),
                constraints
            );
        }
        // Single-pass partitioning into reusable buffers (avoids 3x Vec alloc per node)
        self.side_buf.clear();
        self.south_buf.clear();
        self.north_buf.clear();
        for port in &ports_snapshot {
            let s = port
                .lock()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            if s == side {
                self.side_buf.push(port.clone());
            }
            if s == PortSide::South {
                self.south_buf.push(port.clone());
            }
            if s == PortSide::North {
                self.north_buf.push(port.clone());
            }
        }

        if constraints.is_order_fixed() {
            if timing {
                eprintln!(
                    "crossmin: distribute_ports node={} fixed-order skip",
                    node_id
                );
            }
            return;
        }

        if timing {
            eprintln!(
                "crossmin: distribute_ports node={} side_ports={} south_ports={} north_ports={}",
                node_id,
                self.side_buf.len(),
                self.south_buf.len(),
                self.north_buf.len()
            );
        }
        // Take ownership temporarily to satisfy borrow checker (&mut self + &self.buf)
        let buf = std::mem::take(&mut self.side_buf);
        self.distribute_ports_for_iter(node, &buf, node_id, layer_index, layer_size);
        self.side_buf = buf;

        let buf = std::mem::take(&mut self.south_buf);
        self.distribute_ports_for_iter(node, &buf, node_id, layer_index, layer_size);
        self.south_buf = buf;

        let buf = std::mem::take(&mut self.north_buf);
        self.distribute_ports_for_iter(node, &buf, node_id, layer_index, layer_size);
        self.north_buf = buf;
        self.sort_ports(node);
        if timing {
            eprintln!("crossmin: distribute_ports end node={}", node_id);
        }
    }

    fn distribute_ports_for_iter(
        &mut self,
        node: &LNodeRef,
        ports: &[LPortRef],
        node_id: i32,
        layer_index: usize,
        layer_size: usize,
    ) {
        let timing = *TRACE_CROSSMIN_TIMING;
        if timing {
            eprintln!(
                "crossmin: distribute_ports_for_iter node={} layer={} ports={}",
                node_id,
                layer_index,
                ports.len()
            );
        }
        self.in_layer_ports.clear();
        self.iterate_ports_and_collect_in_layer_ports(node, &ports);
        if timing {
            eprintln!(
                "crossmin: in_layer_ports collected node={} count={}",
                node_id,
                self.in_layer_ports.len()
            );
        }
        if !self.in_layer_ports.is_empty() {
            self.calculate_in_layer_ports_barycenter_values(node, node_id, layer_index, layer_size);
        }
    }

    fn iterate_ports_and_collect_in_layer_ports(&mut self, node: &LNodeRef, ports: &[LPortRef]) {
        self.min_barycenter = 0.0;
        self.max_barycenter = 0.0;
        // Java: final float absurdlyLargeFloat = 2 * layer.getNodes().size() + 1;
        let absurdly_large_float: f32 = (2 * self.layer_size(node) + 1) as f32;
        let timing = *TRACE_CROSSMIN_TIMING;

        'port_iteration: for port in ports {
            let pid = port_id(port);
            if timing {
                eprintln!("crossmin: iterate_ports port_id={}", pid);
            }
            let side = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            if timing {
                eprintln!("crossmin: port_id={} side={:?}", pid, side);
            }
            let north_south_port = matches!(side, PortSide::North | PortSide::South);
            // Java: float sum = 0; — use f32 to match Java's float accumulation
            let mut sum: f32 = 0.0;

            if north_south_port {
                let dummy = port.lock().ok().and_then(|mut port_guard| {
                    port_guard.get_property(InternalProperties::PORT_DUMMY)
                });
                let Some(dummy) = dummy else {
                    continue;
                };
                let contribution =
                    self.deal_with_north_south_ports(absurdly_large_float as f64, port, &dummy);
                sum += contribution as f32;
                if timing {
                    eprintln!(
                        "crossmin: north_south contribution port_id={} sum={}",
                        pid, sum
                    );
                }
            } else {
                let outgoing_edges = connected_outgoing_edges(port);
                for edge in outgoing_edges {
                    let connected_port =
                        edge.lock().ok().and_then(|edge_guard| edge_guard.target());
                    let Some(connected_port) = connected_port else {
                        continue;
                    };
                    if port_same_layer(&connected_port, node) {
                        self.in_layer_ports.push(port.clone());
                        continue 'port_iteration;
                    } else {
                        let pid = port_id(&connected_port);
                        // Java: sum += portRanks[connectedPort.id]; (float += float)
                        sum += self.port_ranks.get(pid).copied().unwrap_or(0.0) as f32;
                    }
                }
                let incoming_edges = connected_incoming_edges(port);
                for edge in incoming_edges {
                    let connected_port =
                        edge.lock().ok().and_then(|edge_guard| edge_guard.source());
                    let Some(connected_port) = connected_port else {
                        continue;
                    };
                    let same_layer = port_same_layer(&connected_port, node);
                    if same_layer {
                        self.in_layer_ports.push(port.clone());
                        continue 'port_iteration;
                    } else {
                        let pid = port_id(&connected_port);
                        // Java: sum -= portRanks[connectedPort.id]; (float -= float)
                        sum -= self.port_ranks.get(pid).copied().unwrap_or(0.0) as f32;
                    }
                }
            }

            // Java: int degree = port.getDegree();
            let degree: i32 = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.degree() as i32)
                .unwrap_or(0);
            let pid = port_id(port);
            self.ensure_port_capacity(pid);
            if degree > 0 {
                // Java: portBarycenter[port.id] = sum / degree; (float / int → float)
                let value = (sum / degree as f32) as f64;
                self.port_barycenter[pid] = value;
                self.min_barycenter = self.min_barycenter.min(value);
                self.max_barycenter = self.max_barycenter.max(value);
            } else if north_south_port {
                self.port_barycenter[pid] = sum as f64;
            }
        }
    }

    fn calculate_in_layer_ports_barycenter_values(
        &mut self,
        node: &LNodeRef,
        node_id: i32,
        layer_index: usize,
        layer_size: usize,
    ) {
        // Java uses int for nodeIndexInLayer and layerSize
        let node_id_usize = if node_id < 0 { 0 } else { node_id as usize };
        let node_index_in_layer: i32 = self
            .node_positions
            .get(layer_index)
            .and_then(|positions| positions.get(node_id_usize))
            .copied()
            .unwrap_or(node_id_usize) as i32
            + 1;
        let layer_size: i32 = layer_size as i32 + 1;
        let in_layer_ports = self.in_layer_ports.clone();
        for port in &in_layer_ports {
            // Java uses int sum and int inLayerConnections
            let mut sum: i32 = 0;
            let mut in_layer_connections: i32 = 0;
            let connected = connected_ports(port);
            for connected_port in connected {
                if port_same_layer(&connected_port, node) {
                    if port_owner_is(&connected_port, node) {
                        sum += node_index_in_layer;
                    } else {
                        sum += self
                            .position_of_node_port_owner_in_layer(&connected_port, layer_index)
                            as i32
                            + 1;
                    }
                    in_layer_connections += 1;
                }
            }
            if in_layer_connections == 0 {
                continue;
            }
            // Java: float barycenter = (float) sum / inLayerConnections;
            let barycenter: f32 = sum as f32 / in_layer_connections as f32;
            let node_index_f: f32 = node_index_in_layer as f32;
            let layer_size_f: f32 = layer_size as f32;
            let side = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            let pid = port_id(port);
            self.ensure_port_capacity(pid);
            // Java stores directly into float[] portBarycenter, so all arithmetic is float
            if side == PortSide::East {
                if barycenter < node_index_f {
                    self.port_barycenter[pid] =
                        (self.min_barycenter as f32 - barycenter) as f64;
                } else {
                    self.port_barycenter[pid] =
                        (self.max_barycenter as f32 + (layer_size_f - barycenter)) as f64;
                }
            } else if side == PortSide::West {
                if barycenter < node_index_f {
                    self.port_barycenter[pid] =
                        (self.max_barycenter as f32 + barycenter) as f64;
                } else {
                    self.port_barycenter[pid] =
                        (self.min_barycenter as f32 - (layer_size_f - barycenter)) as f64;
                }
            }
        }
    }

    fn deal_with_north_south_ports(
        &self,
        absurdly_large_float: f64,
        port: &LPortRef,
        port_dummy: &LNodeRef,
    ) -> f64 {
        let timing = *TRACE_CROSSMIN_TIMING;
        let mut input = false;
        let mut output = false;
        let dummy_ports = port_dummy
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        if timing {
            eprintln!(
                "crossmin: deal_with_north_south_ports dummy_ports={} for port_id={}",
                dummy_ports.len(),
                port_id(port)
            );
        }
        for dummy_port in dummy_ports {
            let origin_matches = dummy_port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN))
                .and_then(|origin| match origin {
                    crate::org::eclipse::elk::alg::layered::options::Origin::LPort(port_ref) => {
                        Some(port_ref)
                    }
                    _ => None,
                })
                .map(|origin| Arc::ptr_eq(&origin, port))
                .unwrap_or(false);
            if timing {
                eprintln!(
                    "crossmin: dummy_port origin_matches={} dummy_port_id={}",
                    origin_matches,
                    port_id(&dummy_port)
                );
            }
            if !origin_matches {
                continue;
            }
            if timing {
                eprintln!(
                    "crossmin: dummy_port check outgoing port_id={}",
                    port_id(&dummy_port)
                );
            }
            let has_outgoing = dummy_port
                .lock()
                .ok()
                .map(|port_guard| !port_guard.outgoing_edges().is_empty())
                .unwrap_or(false);
            if timing {
                eprintln!(
                    "crossmin: dummy_port outgoing done port_id={} has_outgoing={}",
                    port_id(&dummy_port),
                    has_outgoing
                );
            }
            if has_outgoing {
                output = true;
            } else {
                if timing {
                    eprintln!(
                        "crossmin: dummy_port check incoming port_id={}",
                        port_id(&dummy_port)
                    );
                }
                let has_incoming = dummy_port
                    .lock()
                    .ok()
                    .map(|port_guard| !port_guard.incoming_edges().is_empty())
                    .unwrap_or(false);
                if timing {
                    eprintln!(
                        "crossmin: dummy_port incoming done port_id={} has_incoming={}",
                        port_id(&dummy_port),
                        has_incoming
                    );
                }
                if has_incoming {
                    input = true;
                }
            }
        }

        if timing {
            eprintln!(
                "crossmin: deal_with_north_south_ports lock port side start port_id={}",
                port_id(port)
            );
        }
        let side = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        if timing {
            eprintln!(
                "crossmin: deal_with_north_south_ports lock port side done port_id={} side={:?}",
                port_id(port),
                side
            );
        }
        let result = if input && input ^ output {
            if side == PortSide::North {
                if timing {
                    eprintln!(
                        "crossmin: deal_with_north_south_ports position_of start (input-only) port_id={}",
                        port_id(port)
                    );
                }
                let pos = self.position_of(port_dummy);
                if timing {
                    eprintln!(
                        "crossmin: deal_with_north_south_ports position_of (input-only) port_id={} pos={}",
                        port_id(port),
                        pos
                    );
                }
                -(pos as f64)
            } else {
                if timing {
                    eprintln!(
                        "crossmin: deal_with_north_south_ports position_of start (input-only) port_id={}",
                        port_id(port)
                    );
                }
                let pos = self.position_of(port_dummy);
                if timing {
                    eprintln!(
                        "crossmin: deal_with_north_south_ports position_of (input-only) port_id={} pos={}",
                        port_id(port),
                        pos
                    );
                }
                absurdly_large_float - pos as f64
            }
        } else if output && input ^ output {
            if timing {
                eprintln!(
                    "crossmin: deal_with_north_south_ports position_of start (output-only) port_id={}",
                    port_id(port)
                );
            }
            let pos = self.position_of(port_dummy);
            if timing {
                eprintln!(
                    "crossmin: deal_with_north_south_ports position_of (output-only) port_id={} pos={}",
                    port_id(port),
                    pos
                );
            }
            pos as f64 + 1.0
        } else if input && output {
            if side == PortSide::North {
                0.0
            } else {
                absurdly_large_float / 2.0
            }
        } else {
            0.0
        };
        if timing {
            eprintln!(
                "crossmin: deal_with_north_south_ports done port_id={} side={:?} input={} output={} result={}",
                port_id(port),
                side,
                input,
                output,
                result
            );
        }
        result
    }

    fn position_of(&self, node: &LNodeRef) -> usize {
        let layer_index = layer_index(node).unwrap_or(0);
        let node_index = node_id(node);
        self.node_positions
            .get(layer_index)
            .and_then(|positions| positions.get(node_index))
            .copied()
            .unwrap_or(node_index)
    }

    fn position_of_node_port_owner_in_layer(&self, port: &LPortRef, layer_index: usize) -> usize {
        port.lock()
            .ok()
            .and_then(|port_guard| port_guard.node())
            .map(|node| {
                let node_index = node_id(&node);
                self.node_positions
                    .get(layer_index)
                    .and_then(|positions| positions.get(node_index))
                    .copied()
                    .unwrap_or(node_index)
            })
            .unwrap_or_default()
    }

    fn update_node_positions(&mut self, node_order: &[Vec<LNodeRef>], current_index: usize) {
        if let Some(layer_positions) = self.node_positions.get_mut(current_index) {
            for (index, node) in node_order[current_index].iter().enumerate() {
                let node_id = node_id(node);
                if node_id >= layer_positions.len() {
                    layer_positions.resize(node_id + 1, 0);
                }
                layer_positions[node_id] = index;
            }
        }
    }

    fn has_nested_graph(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .and_then(|node_guard| node_guard.nested_graph())
            .is_some()
    }

    fn is_not_first_layer(&self, length: usize, current_index: usize, is_forward: bool) -> bool {
        if is_forward {
            current_index != 0
        } else {
            current_index + 1 < length
        }
    }

    fn port_type_for(&self, is_forward: bool) -> PortType {
        if is_forward {
            PortType::Output
        } else {
            PortType::Input
        }
    }

    fn sort_ports(&mut self, node: &LNodeRef) {
        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        let mut indexed: Vec<(usize, LPortRef)> = ports.into_iter().enumerate().collect();
        indexed.sort_by(|(idx1, port1), (idx2, port2)| {
            let side1 = port1
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            let side2 = port2
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            let ord = if side1 != side2 {
                side1.cmp(&side2)
            } else {
                let bary1 = self
                    .port_barycenter
                    .get(port_id(port1))
                    .copied()
                    .unwrap_or(0.0);
                let bary2 = self
                    .port_barycenter
                    .get(port_id(port2))
                    .copied()
                    .unwrap_or(0.0);
                if bary1 == 0.0 && bary2 == 0.0 {
                    Ordering::Equal
                } else if bary1 == 0.0 {
                    Ordering::Less
                } else if bary2 == 0.0 {
                    Ordering::Greater
                } else {
                    bary1.partial_cmp(&bary2).unwrap_or(Ordering::Equal)
                }
            };

            if ord == Ordering::Equal {
                idx1.cmp(idx2)
            } else {
                ord
            }
        });
        let ports = indexed.into_iter().map(|(_, port)| port).collect();

        if let Ok(mut node_guard) = node.lock() {
            *node_guard.ports_mut() = ports;
            node_guard.cache_port_sides();
        }
    }

    fn layer_size(&self, node: &LNodeRef) -> usize {
        node.lock()
            .ok()
            .and_then(|node_guard| node_guard.layer())
            .and_then(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().len())
            })
            .unwrap_or(0)
    }

    fn ensure_port_capacity(&mut self, port_id: usize) {
        if port_id >= self.port_ranks.len() {
            self.port_ranks.resize(port_id + 1, 0.0);
            self.port_barycenter.resize(port_id + 1, 0.0);
        }
    }
}

impl ISweepPortDistributor for AbstractBarycenterPortDistributor {
    fn distribute_ports_while_sweeping(
        &mut self,
        node_order: &[Vec<LNodeRef>],
        current_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        let timing = *TRACE_CROSSMIN_TIMING;
        self.update_node_positions(node_order, current_index);
        let free_layer = &node_order[current_index];
        let side = if is_forward_sweep {
            PortSide::West
        } else {
            PortSide::East
        };

        if self.is_not_first_layer(node_order.len(), current_index, is_forward_sweep) {
            let fixed_layer_index = if is_forward_sweep {
                current_index - 1
            } else {
                current_index + 1
            };
            let fixed_layer = &node_order[fixed_layer_index];
            let start = if timing {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.calculate_port_ranks(fixed_layer, self.port_type_for(is_forward_sweep));
            if let Some(start) = start {
                eprintln!(
                    "crossmin: port_ranks fixed_layer={} done in {} ms",
                    fixed_layer_index,
                    start.elapsed().as_millis()
                );
            }
            let free_layer_size = free_layer.len();
            for node in free_layer {
                let start = if timing {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                self.distribute_ports(node, side, current_index, free_layer_size);
                if let Some(start) = start {
                    let node_id = node
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.shape().graph_element().id)
                        .unwrap_or(-1);
                    eprintln!(
                        "crossmin: distribute_ports node={} layer={} done in {} ms",
                        node_id,
                        current_index,
                        start.elapsed().as_millis()
                    );
                }
            }

            let start = if timing {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.calculate_port_ranks(free_layer, self.port_type_for(!is_forward_sweep));
            if let Some(start) = start {
                eprintln!(
                    "crossmin: port_ranks free_layer={} done in {} ms",
                    current_index,
                    start.elapsed().as_millis()
                );
            }
            let fixed_layer_size = fixed_layer.len();
            for node in fixed_layer {
                if !self.has_nested_graph(node) {
                    let start = if timing {
                        Some(std::time::Instant::now())
                    } else {
                        None
                    };
                    self.distribute_ports(
                        node,
                        side.opposed(),
                        fixed_layer_index,
                        fixed_layer_size,
                    );
                    if let Some(start) = start {
                        let node_id = node
                            .lock()
                            .ok()
                            .map(|mut node_guard| node_guard.shape().graph_element().id)
                            .unwrap_or(-1);
                        eprintln!(
                            "crossmin: distribute_ports node={} layer={} done in {} ms",
                            node_id,
                            fixed_layer_index,
                            start.elapsed().as_millis()
                        );
                    }
                }
            }
        } else {
            let free_layer_size = free_layer.len();
            for node in free_layer {
                let start = if timing {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                self.distribute_ports(node, side, current_index, free_layer_size);
                if let Some(start) = start {
                    let node_id = node
                        .lock()
                        .ok()
                        .map(|mut node_guard| node_guard.shape().graph_element().id)
                        .unwrap_or(-1);
                    eprintln!(
                        "crossmin: distribute_ports node={} layer={} done in {} ms",
                        node_id,
                        current_index,
                        start.elapsed().as_millis()
                    );
                }
            }
        }

        false
    }
}

impl AbstractBarycenterPortDistributor {
    pub fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType) {
        let mut consumed_rank = 0.0;
        for node in layer {
            consumed_rank += self.calculate_port_ranks_for_node(node, consumed_rank, port_type);
        }
        if *TRACE_PORT_RANKS {
            self.trace_port_ranks(layer);
        }
    }

    fn trace_port_ranks(&self, layer: &[LNodeRef]) {
        for node in layer {
            let (node_id, layer_idx, ports) = if let Ok(mut node_guard) = node.lock() {
                let nid = node_guard.shape().graph_element().id;
                let layer_idx = node_guard
                    .layer()
                    .and_then(|l| l.lock().ok().map(|mut lg| lg.graph_element().id))
                    .unwrap_or(-1);
                let ports = node_guard.ports().clone();
                (nid, layer_idx, ports)
            } else {
                continue;
            };
            for port in &ports {
                if let Ok(mut port_guard) = port.lock() {
                    let pid = port_guard.shape().graph_element().id as usize;
                    let rank = self.port_ranks.get(pid).copied().unwrap_or(0.0);
                    eprintln!(
                        "[PORT_RANK]\t0\t{}\t{}\t{}\t{}",
                        layer_idx, node_id, pid, rank
                    );
                }
            }
        }
    }
}

impl IInitializable for AbstractBarycenterPortDistributor {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if layer_index >= self.node_positions.len() {
            self.node_positions.resize_with(layer_index + 1, Vec::new);
        }
        self.node_positions[layer_index] = vec![0; node_order[layer_index].len()];
    }

    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        let node = &node_order[layer_index][node_index];
        set_node_id(node, node_index);
        if let Some(layer_positions) = self.node_positions.get_mut(layer_index) {
            if node_index >= layer_positions.len() {
                layer_positions.resize(node_index + 1, 0);
            }
            layer_positions[node_index] = node_index;
        }
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        let port = node_order[layer_index][node_index]
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.ports().get(port_index).cloned());
        if let Some(port) = port {
            set_port_id(&port, self.n_ports);
            self.n_ports += 1;
        }
    }

    fn init_after_traversal(&mut self) {
        self.port_ranks = vec![0.0; self.n_ports];
        self.port_barycenter = vec![0.0; self.n_ports];
    }
}

fn set_port_id(port: &LPortRef, value: usize) {
    if let Ok(mut port_guard) = port.lock() {
        port_guard.shape().graph_element().id = value as i32;
    }
}

fn set_node_id(node: &LNodeRef, value: usize) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.shape().graph_element().id = value as i32;
    }
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn port_id(port: &LPortRef) -> usize {
    port.lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn layer_index(node: &LNodeRef) -> Option<usize> {
    node.lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
}

fn port_same_layer(port: &LPortRef, node: &LNodeRef) -> bool {
    let port_node = port.lock().ok().and_then(|port_guard| port_guard.node());
    let Some(port_node) = port_node else {
        return false;
    };
    let port_layer = port_node
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer());
    let node_layer = if Arc::ptr_eq(&port_node, node) {
        port_layer.clone()
    } else {
        node.lock().ok().and_then(|node_guard| node_guard.layer())
    };
    match (port_layer, node_layer) {
        (Some(port_layer), Some(node_layer)) => Arc::ptr_eq(&port_layer, &node_layer),
        _ => false,
    }
}

fn connected_outgoing_edges(
    port: &LPortRef,
) -> Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.outgoing_edges().clone())
        .unwrap_or_default()
}

fn connected_incoming_edges(
    port: &LPortRef,
) -> Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.incoming_edges().clone())
        .unwrap_or_default()
}

fn connected_ports(port: &LPortRef) -> Vec<LPortRef> {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.connected_ports())
        .unwrap_or_default()
}

fn port_owner_is(port: &LPortRef, node: &LNodeRef) -> bool {
    port.lock()
        .ok()
        .and_then(|port_guard| port_guard.node())
        .map(|owner| Arc::ptr_eq(&owner, node))
        .unwrap_or(false)
}
