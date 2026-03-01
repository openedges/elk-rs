use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::p3order::counting::i_initializable::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::counting::{
    CrossingsCounter, HyperedgeCrossingsCounter,
};

pub struct AllCrossingsCounter {
    crossing_counter: CrossingsCounter,
    has_hyper_edges_east_of_index: Vec<bool>,
    hyperedge_crossings_counter: HyperedgeCrossingsCounter,
    in_layer_edge_counts: Vec<i32>,
    has_north_south_ports: Vec<bool>,
    n_ports: i32,
}

static TRACE_CROSSINGS_CALLS: AtomicUsize = AtomicUsize::new(0);
static TRACE_CROSSINGS_BREAKDOWN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSINGS_BREAKDOWN").is_some());

impl AllCrossingsCounter {
    pub fn new(graph: &[Vec<LNodeRef>]) -> Self {
        let length = graph.len();
        AllCrossingsCounter {
            crossing_counter: CrossingsCounter::new(Vec::new()),
            has_hyper_edges_east_of_index: vec![false; length],
            hyperedge_crossings_counter: HyperedgeCrossingsCounter::new(&[], &[], Vec::new()),
            in_layer_edge_counts: vec![0; length],
            has_north_south_ports: vec![false; length],
            n_ports: 0,
        }
    }

    pub fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.crossing_counter.set_snapshot(snapshot);
    }

    pub fn count_all_crossings(&mut self, current_order: &[Vec<LNodeRef>]) -> i32 {
        if current_order.is_empty() {
            return 0;
        }
        let trace = *TRACE_CROSSINGS_BREAKDOWN;
        let trace_this_call = if trace {
            let _ = TRACE_CROSSINGS_CALLS.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            false
        };

        let west = self
            .crossing_counter
            .count_in_layer_crossings_on_side(&current_order[0], PortSide::West);
        let east = self.crossing_counter.count_in_layer_crossings_on_side(
            &current_order[current_order.len() - 1],
            PortSide::East,
        );
        let mut crossings = west + east;
        if trace_this_call {
            eprintln!("rust-crossings: west={} east={}", west, east);
            eprintln!(
                "rust-crossings: has_hyper_edges={:?} has_north_south={:?} in_layer_edge_counts={:?}",
                self.has_hyper_edges_east_of_index,
                self.has_north_south_ports,
                self.in_layer_edge_counts
            );
        }
        for layer_index in 0..current_order.len() {
            let at = self.count_crossings_at(layer_index, current_order, trace_this_call);
            if trace_this_call {
                eprintln!("rust-crossings: layer[{}]={}", layer_index, at);
            }
            crossings += at;
        }
        if trace_this_call {
            eprintln!("rust-crossings: total={}", crossings);
        }
        crossings
    }

    fn count_crossings_at(
        &mut self,
        layer_index: usize,
        current_order: &[Vec<LNodeRef>],
        trace_this_call: bool,
    ) -> i32 {
        let mut total_crossings = 0;
        let mut hyper = 0;
        let mut left_east = 0;
        let mut right_west = 0;
        let mut between = 0;
        let mut north_south = 0;
        let left_layer = &current_order[layer_index];
        if trace_this_call {
            let left_nodes = node_names(left_layer);
            let right_nodes = if layer_index + 1 < current_order.len() {
                node_names(&current_order[layer_index + 1])
            } else {
                String::from("<none>")
            };
            eprintln!(
                "rust-crossings: layer[{}] nodes_left=[{}] nodes_right=[{}]",
                layer_index, left_nodes, right_nodes
            );
        }
        if layer_index + 1 < current_order.len() {
            let right_layer = &current_order[layer_index + 1];
            if self
                .has_hyper_edges_east_of_index
                .get(layer_index)
                .copied()
                .unwrap_or(false)
            {
                hyper = self
                    .hyperedge_crossings_counter
                    .count_crossings(left_layer, right_layer);
                left_east = self
                    .crossing_counter
                    .count_in_layer_crossings_on_side(left_layer, PortSide::East);
                right_west = self
                    .crossing_counter
                    .count_in_layer_crossings_on_side(right_layer, PortSide::West);
                total_crossings = hyper + left_east + right_west;
            } else {
                between = self
                    .crossing_counter
                    .count_crossings_between_layers(left_layer, right_layer);
                total_crossings = between;
            }
        }

        if self
            .has_north_south_ports
            .get(layer_index)
            .copied()
            .unwrap_or(false)
        {
            north_south = self
                .crossing_counter
                .count_north_south_port_crossings_in_layer(left_layer);
            total_crossings += north_south;
        }

        if trace_this_call {
            eprintln!(
                "rust-crossings: layer[{}] detail hyper={} left_east={} right_west={} between={} north_south={} total={}",
                layer_index,
                hyper,
                left_east,
                right_west,
                between,
                north_south,
                total_crossings
            );
        }
        total_crossings
    }
}

fn node_names(layer: &[LNodeRef]) -> String {
    layer
        .iter()
        .map(|node| {
            node.lock()
                .ok()
                .map(|mut node_guard| node_guard.to_string())
                .unwrap_or_else(|| String::from("<poisoned-node>"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl IInitializable for AllCrossingsCounter {
    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        let node = &node_order[layer_index][node_index];
        let node_type = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type())
            .unwrap_or(NodeType::Normal);
        if node_type == NodeType::NorthSouthPort {
            if let Some(flag) = self.has_north_south_ports.get_mut(layer_index) {
                *flag = true;
            }
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
        let Some(port) = port else {
            return;
        };
        set_port_id(&port, self.n_ports);
        self.n_ports += 1;

        let degree = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.outgoing_edges().len() + port_guard.incoming_edges().len())
            .unwrap_or(0);
        if degree > 1 {
            let side = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            if side == PortSide::East {
                if let Some(flag) = self.has_hyper_edges_east_of_index.get_mut(layer_index) {
                    *flag = true;
                }
            } else if side == PortSide::West && layer_index > 0 {
                if let Some(flag) = self.has_hyper_edges_east_of_index.get_mut(layer_index - 1) {
                    *flag = true;
                }
            }
        }
    }

    fn init_at_edge_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        _edge_index: usize,
        edge: &LEdgeRef,
        node_order: &[Vec<LNodeRef>],
    ) {
        let port = node_order[layer_index][node_index]
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.ports().get(port_index).cloned());
        let Some(port) = port else {
            return;
        };
        let source = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
        if let Some(source) = source {
            if Arc::ptr_eq(&source, &port) {
                let source_layer = source
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node())
                    .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
                let target_layer = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
                if let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) {
                    if Arc::ptr_eq(&source_layer, &target_layer) {
                        if let Some(count) = self.in_layer_edge_counts.get_mut(layer_index) {
                            *count += 1;
                        }
                    }
                }
            }
        }
    }

    fn init_after_traversal(&mut self) {
        let port_positions = vec![0; self.n_ports as usize];
        let port_positions_hyper = vec![0; self.n_ports as usize];
        self.hyperedge_crossings_counter = HyperedgeCrossingsCounter::new(
            &self.in_layer_edge_counts,
            &self.has_north_south_ports,
            port_positions_hyper,
        );
        self.crossing_counter = CrossingsCounter::new(port_positions);
    }
}

fn set_port_id(port: &LPortRef, value: i32) {
    if let Ok(mut port_guard) = port.lock() {
        port_guard.shape().graph_element().id = value;
    }
}
