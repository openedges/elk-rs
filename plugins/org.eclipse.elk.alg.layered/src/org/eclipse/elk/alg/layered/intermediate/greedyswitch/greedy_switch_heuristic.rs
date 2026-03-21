use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::crossing_matrix_filler::CrossingMatrixFiller;
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::switch_decider::{
    CrossingCountSide, ParentCrossingContext, SwitchDecider,
};
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;

pub struct ParentContext {
    parent_node_order: Vec<Vec<LNodeRef>>,
    parent_port_positions: Vec<i32>,
    parent_node: LNodeRef,
}

pub struct GreedySwitchHeuristic {
    greedy_switch_type: CrossMinType,
    port_positions: Vec<i32>,
    n_ports: usize,
    has_parent: bool,
    dont_sweep_into: bool,
    parent_context: Option<ParentContext>,
}

impl GreedySwitchHeuristic {
    pub fn new(greedy_type: CrossMinType, has_parent: bool) -> Self {
        GreedySwitchHeuristic {
            greedy_switch_type: greedy_type,
            port_positions: Vec::new(),
            n_ports: 0,
            has_parent,
            dont_sweep_into: false,
            parent_context: None,
        }
    }

    pub fn set_dont_sweep_into(&mut self, dont_sweep_into: bool) {
        self.dont_sweep_into = dont_sweep_into;
    }

    pub fn update_parent_context(
        &mut self,
        parent_node_order: Vec<Vec<LNodeRef>>,
        parent_port_positions: Vec<i32>,
        parent_node: LNodeRef,
    ) {
        self.parent_context = Some(ParentContext {
            parent_node_order,
            parent_port_positions,
            parent_node,
        });
    }

    fn set_up(
        &self,
        order: &[Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
    ) -> Option<SwitchDecider> {
        if free_layer_index >= order.len() {
            return None;
        }
        let side = if forward_sweep {
            CrossingCountSide::West
        } else {
            CrossingCountSide::East
        };
        let crossing_matrix_filler =
            CrossingMatrixFiller::new(self.greedy_switch_type, order, free_layer_index, side);
        let one_sided = self.greedy_switch_type == CrossMinType::OneSidedGreedySwitch;
        let free_layer = &order[free_layer_index];
        let free_layer_first_is_external_port = free_layer
            .first()
            .and_then(|node| node.lock_ok().map(|node_guard| node_guard.node_type()))
            .map(|node_type| node_type == NodeType::ExternalPort)
            .unwrap_or(false);

        let mut parent_context = None;
        let mut count_parent_crossings = !one_sided
            && self.has_parent
            && !self.dont_sweep_into
            && free_layer_first_is_external_port;

        if count_parent_crossings {
            if let Some(parent) = self.parent_context.as_ref() {
                if let Some(parent_layer_index) = layer_index_of(&parent.parent_node) {
                    let right_most_layer = free_layer_index + 1 == order.len();
                    parent_context = Some(ParentCrossingContext::new(
                        parent.parent_node_order.clone(),
                        parent.parent_port_positions.clone(),
                        parent_layer_index,
                        right_most_layer,
                    ));
                } else {
                    count_parent_crossings = false;
                }
            } else {
                count_parent_crossings = false;
            }
        }

        Some(SwitchDecider::new(
            free_layer,
            crossing_matrix_filler,
            &self.port_positions,
            parent_context,
            count_parent_crossings,
        ))
    }

    fn continue_switching_until_no_improvement_in_layer(
        &self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        switch_decider: &mut SwitchDecider,
    ) -> bool {
        let mut improved = false;
        let trace = ElkTrace::global().greedy_switch;
        let mut iterations = 0usize;
        loop {
            let continue_switching =
                self.sweep_downward_in_layer(order, free_layer_index, switch_decider);
            improved |= continue_switching;
            if trace {
                eprintln!(
                    "greedy_switch: layer={} iter={} continue={}",
                    free_layer_index, iterations, continue_switching
                );
            }
            iterations = iterations.wrapping_add(1);
            if !continue_switching {
                break;
            }
        }
        improved
    }

    fn sweep_downward_in_layer(
        &self,
        order: &mut [Vec<LNodeRef>],
        layer_index: usize,
        switch_decider: &mut SwitchDecider,
    ) -> bool {
        let mut continue_switching = false;
        let length = order.get(layer_index).map(|layer| layer.len()).unwrap_or(0);
        for upper_node_index in 0..length.saturating_sub(1) {
            let lower_node_index = upper_node_index + 1;
            continue_switching |= self.switch_if_improves(
                order,
                layer_index,
                upper_node_index,
                lower_node_index,
                switch_decider,
            );
        }
        continue_switching
    }

    fn switch_if_improves(
        &self,
        order: &mut [Vec<LNodeRef>],
        layer_index: usize,
        upper_node_index: usize,
        lower_node_index: usize,
        switch_decider: &mut SwitchDecider,
    ) -> bool {
        let layer = match order.get(layer_index) {
            Some(layer) => layer,
            None => return false,
        };
        let upper_node = match layer.get(upper_node_index) {
            Some(node) => node.clone(),
            None => return false,
        };
        let lower_node = match layer.get(lower_node_index) {
            Some(node) => node.clone(),
            None => return false,
        };

        if switch_decider.does_switch_reduce_crossings(&upper_node, &lower_node) {
            self.exchange_nodes(
                order,
                layer_index,
                upper_node_index,
                lower_node_index,
                switch_decider,
            );
            return true;
        }
        false
    }

    fn exchange_nodes(
        &self,
        order: &mut [Vec<LNodeRef>],
        layer_index: usize,
        index_one: usize,
        index_two: usize,
        switch_decider: &mut SwitchDecider,
    ) {
        let layer = match order.get_mut(layer_index) {
            Some(layer) => layer,
            None => return,
        };
        if index_one >= layer.len() || index_two >= layer.len() {
            return;
        }
        let upper_node = layer[index_one].clone();
        let lower_node = layer[index_two].clone();
        switch_decider.notify_of_switch(&upper_node, &lower_node);
        layer.swap(index_one, index_two);
    }

    fn start_index(&self, forward_sweep: bool, length: usize) -> usize {
        if forward_sweep {
            0
        } else {
            length.saturating_sub(1)
        }
    }
}

impl ICrossingMinimizationHeuristic for GreedySwitchHeuristic {
    fn always_improves(&self) -> bool {
        self.greedy_switch_type != CrossMinType::OneSidedGreedySwitch
    }

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool, _random: &mut Random) -> bool {
        let start_index = self.start_index(forward_sweep, order.len());
        if let Some(mut switch_decider) = self.set_up(order, start_index, forward_sweep) {
            return self.sweep_downward_in_layer(order, start_index, &mut switch_decider);
        }
        false
    }

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        _is_first_sweep: bool,
        _random: &mut Random,
    ) -> bool {
        if let Some(mut switch_decider) = self.set_up(order, free_layer_index, forward_sweep) {
            return self.continue_switching_until_no_improvement_in_layer(
                order,
                free_layer_index,
                &mut switch_decider,
            );
        }
        false
    }

    fn is_deterministic(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl IInitializable for GreedySwitchHeuristic {
    fn init_at_port_level(
        &mut self,
        _layer_index: usize,
        _node_index: usize,
        _port_index: usize,
        _node_order: &[Vec<LNodeRef>],
    ) {
        self.n_ports += 1;
    }

    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Some(node) = node_order.get(layer_index).and_then(|layer| layer.first()) {
            if let Some(layer_ref) = node.lock_ok().and_then(|node_guard| node_guard.layer()) {
                if let Some(mut layer_guard) = layer_ref.lock_ok() {
                    layer_guard.graph_element().id = layer_index as i32;
                }
            }
        }
    }

    fn init_after_traversal(&mut self) {
        self.port_positions = vec![0; self.n_ports];
    }
}

fn layer_index_of(node: &LNodeRef) -> Option<usize> {
    node.lock_ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock_ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
}
