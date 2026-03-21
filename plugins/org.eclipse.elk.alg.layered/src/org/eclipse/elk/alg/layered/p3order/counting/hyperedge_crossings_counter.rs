use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rustc_hash::FxHashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, LayerRef};

pub struct HyperedgeCrossingsCounter {
    port_positions: Vec<i32>,
}

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

static TRACE_HYPER_CALLS: AtomicUsize = AtomicUsize::new(0);

impl HyperedgeCrossingsCounter {
    pub fn new(
        _in_layer_edge_count: &[i32],
        _has_north_south_ports: &[bool],
        port_positions: Vec<i32>,
    ) -> Self {
        HyperedgeCrossingsCounter { port_positions }
    }

    pub fn count_crossings(&mut self, left_layer: &[LNodeRef], right_layer: &[LNodeRef]) -> i32 {
        let trace = ElkTrace::global().crossings_breakdown;
        let trace_call = if trace {
            Some(TRACE_HYPER_CALLS.fetch_add(1, Ordering::SeqCst))
        } else {
            None
        };
        if left_layer.is_empty() || right_layer.is_empty() {
            return 0;
        }

        let mut source_count = 0i32;
        for node in left_layer {
            let ports = node
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in ports {
                let (outgoing, port_name) = port
                    .lock_ok()
                    .map(|port_guard| {
                        let edges = port_guard.outgoing_edges().clone();
                        let name = if trace_call.is_some_and(|c| c < 64) {
                            Some(port_guard.to_string())
                        } else {
                            None
                        };
                        (edges, name)
                    })
                    .unwrap_or_default();
                let mut port_edges = 0;
                for edge in outgoing {
                    if !edge_is_in_layer(&edge) {
                        port_edges += 1;
                    }
                }
                if port_edges > 0 {
                    set_port_position(&mut self.port_positions, &port, source_count);
                    if let (Some(call_idx), Some(name)) = (trace_call, &port_name) {
                        if call_idx < 64 {
                            eprintln!(
                                "rust-hyper: call={} source_pos {} <- {}",
                                call_idx, source_count, name
                            );
                        }
                    }
                    source_count += 1;
                }
            }
        }

        let mut target_count = 0i32;
        let do_trace_name = trace_call.is_some_and(|c| c < 64);
        for node in right_layer {
            let ports = node
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();

            // Single-lock extraction: (side, incoming_edges) per port
            let mut north_input_ports = 0i32;
            for port in &ports {
                let (side, incoming) = port
                    .lock_ok()
                    .map(|port_guard| (port_guard.side(), port_guard.incoming_edges().clone()))
                    .unwrap_or((PortSide::Undefined, Vec::new()));
                if side == PortSide::North {
                    for edge in incoming {
                        if !edge_is_in_layer(&edge) {
                            north_input_ports += 1;
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            let mut other_input_ports = 0i32;
            for port in ports.iter().rev() {
                // Single lock: extract side + incoming_edges + optional name
                let (incoming, side, port_name) = port
                    .lock_ok()
                    .map(|port_guard| {
                        let inc = port_guard.incoming_edges().clone();
                        let s = port_guard.side();
                        let name = if do_trace_name {
                            Some(port_guard.to_string())
                        } else {
                            None
                        };
                        (inc, s, name)
                    })
                    .unwrap_or((Vec::new(), PortSide::Undefined, None));
                let mut port_edges = 0;
                for edge in incoming {
                    if !edge_is_in_layer(&edge) {
                        port_edges += 1;
                    }
                }
                if port_edges > 0 {
                    if side == PortSide::North {
                        set_port_position(&mut self.port_positions, port, target_count);
                        if let (Some(call_idx), Some(name)) = (trace_call, &port_name) {
                            eprintln!(
                                "rust-hyper: call={} target_pos {} <- {}",
                                call_idx, target_count, name
                            );
                        }
                        target_count += 1;
                    } else {
                        let assigned = target_count + north_input_ports + other_input_ports;
                        set_port_position(&mut self.port_positions, port, assigned);
                        if let (Some(call_idx), Some(name)) = (trace_call, &port_name) {
                            eprintln!(
                                "rust-hyper: call={} target_pos {} <- {}",
                                call_idx, assigned, name
                            );
                        }
                        other_input_ports += 1;
                    }
                }
            }
            target_count += other_input_ports;
        }

        let mut port_to_hyperedge: FxHashMap<usize, usize> = FxHashMap::default();
        let mut hyperedges: Vec<Hyperedge> = Vec::new();
        let mut active: Vec<bool> = Vec::new();

        for node in left_layer {
            let ports = node
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for source_port in ports {
                let outgoing = source_port
                    .lock_ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing {
                    let target_port = edge.lock_ok().and_then(|edge_guard| edge_guard.target());
                    let Some(target_port) = target_port else {
                        continue;
                    };
                    if edge_is_in_layer(&edge) {
                        continue;
                    }
                    let source_key = port_ptr_id(&source_port);
                    let target_key = port_ptr_id(&target_port);
                    let source_he = port_to_hyperedge.get(&source_key).copied();
                    let target_he = port_to_hyperedge.get(&target_key).copied();
                    match (source_he, target_he) {
                        (None, None) => {
                            let id = hyperedges.len();
                            let mut hyperedge = Hyperedge::new(source_key);
                            hyperedge.edges.push(edge.clone());
                            hyperedge.ports.push(source_port.clone());
                            hyperedge.ports.push(target_port.clone());
                            hyperedges.push(hyperedge);
                            active.push(true);
                            port_to_hyperedge.insert(source_key, id);
                            port_to_hyperedge.insert(target_key, id);
                        }
                        (None, Some(target_id)) => {
                            if let Some(hyperedge) = hyperedges.get_mut(target_id) {
                                hyperedge.edges.push(edge.clone());
                                hyperedge.ports.push(source_port.clone());
                                port_to_hyperedge.insert(source_key, target_id);
                            }
                        }
                        (Some(source_id), None) => {
                            if let Some(hyperedge) = hyperedges.get_mut(source_id) {
                                hyperedge.edges.push(edge.clone());
                                hyperedge.ports.push(target_port.clone());
                                port_to_hyperedge.insert(target_key, source_id);
                            }
                        }
                        (Some(source_id), Some(target_id)) => {
                            if source_id == target_id {
                                if let Some(hyperedge) = hyperedges.get_mut(source_id) {
                                    hyperedge.edges.push(edge.clone());
                                }
                            } else if source_id < hyperedges.len() && target_id < hyperedges.len() {
                                let (source_he, target_he) = if source_id < target_id {
                                    let (left, right) = hyperedges.split_at_mut(target_id);
                                    (&mut left[source_id], &mut right[0])
                                } else {
                                    let (left, right) = hyperedges.split_at_mut(source_id);
                                    (&mut right[0], &mut left[target_id])
                                };
                                source_he.edges.push(edge.clone());
                                for port in &target_he.ports {
                                    port_to_hyperedge.insert(port_ptr_id(port), source_id);
                                }
                                source_he.edges.append(&mut target_he.edges);
                                source_he.ports.append(&mut target_he.ports);
                                active[target_id] = false;
                            }
                        }
                    }
                }
            }
        }

        let mut hyperedge_list: Vec<Hyperedge> = active
            .iter()
            .enumerate()
            .filter(|(_, &is_active)| is_active)
            .filter_map(|(id, _)| hyperedges.get(id).cloned())
            .collect();

        let left_layer_ref = left_layer
            .first()
            .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.layer()));
        let right_layer_ref = right_layer
            .first()
            .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.layer()));

        for hyperedge in &mut hyperedge_list {
            hyperedge.upper_left = source_count;
            hyperedge.upper_right = target_count;
            for port in &hyperedge.ports {
                let pos = port_position(&self.port_positions, port);
                if port_layer(port)
                    .as_ref()
                    .zip(left_layer_ref.as_ref())
                    .is_some_and(|(a, b)| Arc::ptr_eq(a, b))
                {
                    hyperedge.upper_left = hyperedge.upper_left.min(pos);
                    hyperedge.lower_left = hyperedge.lower_left.max(pos);
                } else if port_layer(port)
                    .as_ref()
                    .zip(right_layer_ref.as_ref())
                    .is_some_and(|(a, b)| Arc::ptr_eq(a, b))
                {
                    hyperedge.upper_right = hyperedge.upper_right.min(pos);
                    hyperedge.lower_right = hyperedge.lower_right.max(pos);
                }
            }
        }

        hyperedge_list.sort_by(|a, b| a.compare(b));

        let mut south_sequence: Vec<i32> = Vec::with_capacity(hyperedge_list.len());
        let mut compress_deltas = vec![0i32; (target_count + 1) as usize];
        for hyperedge in &hyperedge_list {
            south_sequence.push(hyperedge.upper_right);
            compress_deltas[hyperedge.upper_right as usize] = 1;
        }
        let mut delta = 0;
        for entry in compress_deltas.iter_mut() {
            if *entry == 1 {
                *entry = delta;
            } else {
                delta -= 1;
            }
        }
        let mut q = 0;
        for value in south_sequence.iter_mut() {
            let idx = *value as usize;
            *value += compress_deltas[idx];
            q = q.max(*value + 1);
        }

        let mut first_index = 1;
        while first_index < q {
            first_index *= 2;
        }
        let tree_size = 2 * first_index - 1;
        first_index -= 1;
        let mut tree = vec![0i32; tree_size as usize];

        let mut crossings = 0;
        let mut straight_crossings = 0;
        for value in &south_sequence {
            let mut index = *value + first_index;
            tree[index as usize] += 1;
            while index > 0 {
                if index % 2 > 0 {
                    let delta = tree[(index + 1) as usize];
                    crossings += delta;
                    straight_crossings += delta;
                }
                index = (index - 1) / 2;
                tree[index as usize] += 1;
            }
        }

        let mut left_corners: Vec<HyperedgeCorner> = Vec::with_capacity(hyperedge_list.len() * 2);
        for hyperedge in &hyperedge_list {
            left_corners.push(HyperedgeCorner {
                identity_hash: hyperedge.identity_hash,
                position: hyperedge.upper_left,
                opposite_position: hyperedge.lower_left,
                corner_type: CornerType::Upper,
            });
            left_corners.push(HyperedgeCorner {
                identity_hash: hyperedge.identity_hash,
                position: hyperedge.lower_left,
                opposite_position: hyperedge.upper_left,
                corner_type: CornerType::Lower,
            });
        }
        left_corners.sort_by(|a, b| a.compare(b));

        let mut open_hyperedges = 0;
        let mut left_overlap_crossings = 0;
        for corner in &left_corners {
            match corner.corner_type {
                CornerType::Upper => open_hyperedges += 1,
                CornerType::Lower => {
                    open_hyperedges -= 1;
                    crossings += open_hyperedges;
                    left_overlap_crossings += open_hyperedges;
                }
            }
        }

        let mut right_corners: Vec<HyperedgeCorner> = Vec::with_capacity(hyperedge_list.len() * 2);
        for hyperedge in &hyperedge_list {
            right_corners.push(HyperedgeCorner {
                identity_hash: hyperedge.identity_hash,
                position: hyperedge.upper_right,
                opposite_position: hyperedge.lower_right,
                corner_type: CornerType::Upper,
            });
            right_corners.push(HyperedgeCorner {
                identity_hash: hyperedge.identity_hash,
                position: hyperedge.lower_right,
                opposite_position: hyperedge.upper_right,
                corner_type: CornerType::Lower,
            });
        }
        right_corners.sort_by(|a, b| a.compare(b));

        open_hyperedges = 0;
        let mut right_overlap_crossings = 0;
        for corner in &right_corners {
            match corner.corner_type {
                CornerType::Upper => open_hyperedges += 1,
                CornerType::Lower => {
                    open_hyperedges -= 1;
                    crossings += open_hyperedges;
                    right_overlap_crossings += open_hyperedges;
                }
            }
        }

        if let Some(call_idx) = trace_call {
            if call_idx < 64 {
                eprintln!(
                    "rust-hyper: call={} source_count={} target_count={} hyperedges={} straight={} left_overlap={} right_overlap={} total={}",
                    call_idx,
                    source_count,
                    target_count,
                    hyperedge_list.len(),
                    straight_crossings,
                    left_overlap_crossings,
                    right_overlap_crossings,
                    crossings
                );
                for (idx, hyperedge) in hyperedge_list.iter().enumerate() {
                    eprintln!(
                        "rust-hyper: call={} edge[{}] ul={} ll={} ur={} lr={} ports={} edges={}",
                        call_idx,
                        idx,
                        hyperedge.upper_left,
                        hyperedge.lower_left,
                        hyperedge.upper_right,
                        hyperedge.lower_right,
                        hyperedge.ports.len(),
                        hyperedge.edges.len()
                    );
                }
            }
        }

        crossings
    }
}

#[derive(Clone)]
struct Hyperedge {
    identity_hash: usize,
    edges: Vec<LEdgeRef>,
    ports: Vec<LPortRef>,
    upper_left: i32,
    lower_left: i32,
    upper_right: i32,
    lower_right: i32,
}

impl Hyperedge {
    fn new(identity_hash: usize) -> Self {
        Hyperedge {
            identity_hash,
            edges: Vec::new(),
            ports: Vec::new(),
            upper_left: 0,
            lower_left: 0,
            upper_right: 0,
            lower_right: 0,
        }
    }

    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.upper_left
            .cmp(&other.upper_left)
            .then_with(|| self.upper_right.cmp(&other.upper_right))
            .then_with(|| self.identity_hash.cmp(&other.identity_hash))
    }
}

#[derive(Clone, Copy)]
enum CornerType {
    Upper,
    Lower,
}

#[derive(Clone, Copy)]
struct HyperedgeCorner {
    identity_hash: usize,
    position: i32,
    opposite_position: i32,
    corner_type: CornerType,
}

impl HyperedgeCorner {
    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.position
            .cmp(&other.position)
            .then_with(|| self.opposite_position.cmp(&other.opposite_position))
            .then_with(|| self.identity_hash.cmp(&other.identity_hash))
            .then_with(|| match (self.corner_type, other.corner_type) {
                (CornerType::Upper, CornerType::Lower) => std::cmp::Ordering::Less,
                (CornerType::Lower, CornerType::Upper) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            })
    }
}

fn edge_is_in_layer(edge: &LEdgeRef) -> bool {
    let (source_layer, target_layer) = edge
        .lock_ok()
        .map(|edge_guard| {
            let source_layer = edge_guard
                .source()
                .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()))
                .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.layer()));
            let target_layer = edge_guard
                .target()
                .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()))
                .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.layer()));
            (source_layer, target_layer)
        })
        .unwrap_or((None, None));
    if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
        Arc::ptr_eq(&source_layer, &target_layer)
    } else {
        if ElkTrace::global().crossings_breakdown {
            eprintln!("rust-crossings: edge_is_in_layer missing layer endpoint");
        }
        false
    }
}

fn port_ptr_id(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn port_id(port: &LPortRef) -> usize {
    port.lock_ok()
        .map(|mut port_guard| port_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn set_port_position(port_positions: &mut Vec<i32>, port: &LPortRef, position: i32) {
    let pid = port_id(port);
    if pid >= port_positions.len() {
        port_positions.resize(pid + 1, 0);
    }
    port_positions[pid] = position;
}

fn port_position(port_positions: &[i32], port: &LPortRef) -> i32 {
    let pid = port_id(port);
    *port_positions.get(pid).unwrap_or(&0)
}

fn port_layer(port: &LPortRef) -> Option<LayerRef> {
    port.lock_ok()
        .and_then(|port_guard| port_guard.node())
        .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.layer()))
}
