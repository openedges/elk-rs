use std::sync::LazyLock;

static TRACE_GREEDY_PORTS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_GREEDY_PORTS").is_some());

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::BetweenLayerEdgeTwoNodeCrossingsCounter;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::p3order::counting::{CrossingsCounter, IInitializable};
use crate::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;

pub struct GreedyPortDistributor {
    crossings_counter: Option<CrossingsCounter>,
    n_ports: usize,
}

impl GreedyPortDistributor {
    pub fn new() -> Self {
        GreedyPortDistributor {
            crossings_counter: None,
            n_ports: 0,
        }
    }

    fn crossings_counter_mut(&mut self) -> &mut CrossingsCounter {
        self.crossings_counter
            .as_mut()
            .expect("crossings counter not initialized")
    }

    fn initialize(
        &mut self,
        node_order: &[Vec<LNodeRef>],
        current_index: usize,
        is_forward_sweep: bool,
    ) {
        if is_forward_sweep && current_index > 0 {
            self.init_for_layers(&node_order[current_index - 1], &node_order[current_index]);
        } else if !is_forward_sweep && current_index + 1 < node_order.len() {
            self.init_for_layers(&node_order[current_index], &node_order[current_index + 1]);
        } else {
            let side = if is_forward_sweep {
                PortSide::West
            } else {
                PortSide::East
            };
            self.crossings_counter_mut()
                .init_port_positions_for_in_layer_crossings(&node_order[current_index], side);
        }
    }

    fn init_for_layers(&mut self, left_layer: &[LNodeRef], right_layer: &[LNodeRef]) {
        self.crossings_counter_mut()
            .init_for_counting_between(left_layer, right_layer);
    }

    fn distribute_ports_in_layer(
        &mut self,
        node_order: &[Vec<LNodeRef>],
        current_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        let side = if is_forward_sweep {
            PortSide::West
        } else {
            PortSide::East
        };
        let mut improved = false;

        for node in &node_order[current_index] {
            let port_constraints = node
                .lock()
                .ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
                })
                .unwrap_or(PortConstraints::Undefined);
            if port_constraints.is_order_fixed() {
                continue;
            }

            let ports_on_side = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.port_side_view(side))
                .unwrap_or_default();
            let nested_graph = node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.nested_graph());
            let use_hierarchical = !ports_on_side.is_empty() && nested_graph.is_some();

            let mut hierarchical_counter = if use_hierarchical {
                nested_graph
                    .as_ref()
                    .and_then(|graph| graph.lock().ok())
                    .map(|graph_guard| graph_guard.to_node_array())
                    .map(|inner_graph| {
                        let free_layer_index = if is_forward_sweep {
                            0
                        } else {
                            inner_graph.len() - 1
                        };
                        BetweenLayerEdgeTwoNodeCrossingsCounter::new(inner_graph, free_layer_index)
                    })
            } else {
                None
            };

            improved |= self.distribute_ports_on_node(node, side, &mut hierarchical_counter);
        }

        improved
    }

    fn distribute_ports_on_node(
        &mut self,
        node: &LNodeRef,
        side: PortSide,
        hierarchical_counter: &mut Option<BetweenLayerEdgeTwoNodeCrossingsCounter>,
    ) -> bool {
        let (mut ports, mut indices) = self.ports_for_side(node, side);
        if ports.is_empty() {
            return false;
        }

        let mut improved = false;
        let mut continue_switching;
        let trace = *TRACE_GREEDY_PORTS;
        let mut iterations = 0usize;
        loop {
            continue_switching = false;
            for i in 0..ports.len().saturating_sub(1) {
                let upper_port = ports[i].clone();
                let lower_port = ports[i + 1].clone();
                if self.switching_decreases_crossings(
                    &upper_port,
                    &lower_port,
                    hierarchical_counter.as_mut(),
                ) {
                    improved = true;
                    self.switch_ports(node, &mut ports, &mut indices, i, i + 1);
                    continue_switching = true;
                }
            }
            if trace {
                let node_id = node
                    .lock()
                    .ok()
                    .map(|mut node_guard| node_guard.shape().graph_element().id)
                    .unwrap_or(-1);
                eprintln!(
                    "greedy_ports: node={} side={:?} iter={} continue={}",
                    node_id, side, iterations, continue_switching
                );
            }
            iterations = iterations.wrapping_add(1);
            if !continue_switching {
                break;
            }
        }

        improved
    }

    fn ports_for_side(&self, node: &LNodeRef, side: PortSide) -> (Vec<LPortRef>, Vec<usize>) {
        let mut ports = Vec::new();
        let mut indices = Vec::new();
        if let Ok(mut node_guard) = node.lock() {
            ports = node_guard.port_side_view(side);
            indices = node_guard
                .ports()
                .iter()
                .enumerate()
                .filter(|(_, port)| {
                    port.lock()
                        .ok()
                        .map(|port_guard| port_guard.side() == side)
                        .unwrap_or(false)
                })
                .map(|(index, _)| index)
                .collect();
        }

        if side == PortSide::South || side == PortSide::West {
            ports.reverse();
            indices.reverse();
        }

        (ports, indices)
    }

    fn switching_decreases_crossings(
        &mut self,
        upper_port: &LPortRef,
        lower_port: &LPortRef,
        hierarchical_counter: Option<&mut BetweenLayerEdgeTwoNodeCrossingsCounter>,
    ) -> bool {
        let crossings = self
            .crossings_counter_mut()
            .count_crossings_between_ports_in_both_orders(upper_port, lower_port);
        let mut upper_lower_crossings = crossings.first;
        let mut lower_upper_crossings = crossings.second;

        if let Some(counter) = hierarchical_counter {
            let upper_node = upper_port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY));
            let lower_node = lower_port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY));
            if let (Some(upper_node), Some(lower_node)) = (upper_node, lower_node) {
                counter.count_both_side_crossings(&upper_node, &lower_node);
                upper_lower_crossings += counter.upper_lower_crossings();
                lower_upper_crossings += counter.lower_upper_crossings();
            }
        }

        upper_lower_crossings > lower_upper_crossings
    }

    fn switch_ports(
        &mut self,
        node: &LNodeRef,
        ports: &mut [LPortRef],
        indices: &mut [usize],
        top_index: usize,
        bottom_index: usize,
    ) {
        if top_index >= ports.len() || bottom_index >= ports.len() {
            return;
        }

        self.crossings_counter_mut()
            .switch_ports(&ports[top_index], &ports[bottom_index]);
        let top_pos = indices.get(top_index).copied();
        let bottom_pos = indices.get(bottom_index).copied();

        ports.swap(top_index, bottom_index);
        indices.swap(top_index, bottom_index);

        if let (Some(top_pos), Some(bottom_pos)) = (top_pos, bottom_pos) {
            if let Ok(mut node_guard) = node.lock() {
                let node_ports = node_guard.ports_mut();
                if top_pos < node_ports.len() && bottom_pos < node_ports.len() {
                    node_ports.swap(top_pos, bottom_pos);
                }
            }
        }
    }
}

impl Default for GreedyPortDistributor {
    fn default() -> Self {
        Self::new()
    }
}

impl ISweepPortDistributor for GreedyPortDistributor {
    fn distribute_ports_while_sweeping(
        &mut self,
        node_order: &[Vec<LNodeRef>],
        current_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        self.initialize(node_order, current_index, is_forward_sweep);
        self.distribute_ports_in_layer(node_order, current_index, is_forward_sweep)
    }
}

impl IInitializable for GreedyPortDistributor {
    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        let node = &node_order[layer_index][node_index];
        if let Ok(mut node_guard) = node.lock() {
            node_guard.shape().graph_element().id = node_index as i32;
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
            if let Ok(mut port_guard) = port.lock() {
                port_guard.shape().graph_element().id = self.n_ports as i32;
            }
            self.n_ports += 1;
        }
    }

    fn init_after_traversal(&mut self) {
        self.crossings_counter = Some(CrossingsCounter::new(vec![0; self.n_ports]));
    }
}
