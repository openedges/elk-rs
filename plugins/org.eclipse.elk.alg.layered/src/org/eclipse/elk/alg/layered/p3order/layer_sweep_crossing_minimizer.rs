#![allow(clippy::mutable_key_type)]

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::sync::{Arc, LazyLock};

static TRACE_CROSSMIN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());
static TRACE_CROSSMIN_TIMING: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN_TIMING").is_some());
static TRACE_CROSSMIN_STATS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN_STATS").is_some());
static DEBUG_CROSSMIN_FORCE_SWEEP: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("ELK_DEBUG_CROSSMIN_FORCE_SWEEP").ok());

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::preserveorder::{
    ModelOrderNodeComparator, ModelOrderPortComparator,
};
use crate::org::eclipse::elk::alg::layered::intermediate::sort_by_input_model_processor::SortByInputModelProcessor;
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions, LongEdgeOrderingStrategy,
    OrderingStrategy,
};
use crate::org::eclipse::elk::alg::layered::p3order::{
    in_north_south_east_west_order, GraphInfoHolder,
};
use crate::org::eclipse::elk::alg::layered::p3order::random_trace;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CrossMinType {
    Barycenter,
    OneSidedGreedySwitch,
    TwoSidedGreedySwitch,
    Median,
}

#[derive(Debug)]
enum MinimizingMethod {
    CompareRandomized,
    NoCounter,
    WithCounter,
}

pub struct LayerSweepCrossingMinimizer {
    cross_min_type: CrossMinType,
    graph_info_holders: Vec<GraphInfoHolder>,
    graphs_whose_node_order_changed: BTreeSet<usize>,
    random: Random,
    random_seed: u64,
}

impl LayerSweepCrossingMinimizer {
    pub fn new(cross_min_type: CrossMinType) -> Self {
        LayerSweepCrossingMinimizer {
            cross_min_type,
            graph_info_holders: Vec::new(),
            graphs_whose_node_order_changed: BTreeSet::new(),
            random: Random::default(),
            random_seed: 0,
        }
    }

    fn choose_minimizing_method(&self, root_index: usize) -> MinimizingMethod {
        let parent = &self.graph_info_holders[root_index];
        if !parent.cross_min_deterministic() {
            MinimizingMethod::CompareRandomized
        } else if parent.cross_min_always_improves() {
            MinimizingMethod::NoCounter
        } else {
            MinimizingMethod::WithCounter
        }
    }

    fn minimize_crossings(&mut self, graphs_to_sweep_on: &[usize], method: MinimizingMethod) {
        for &index in graphs_to_sweep_on {
            let has_layers = self
                .graph_info_holders
                .get(index)
                .map(|g| !g.current_node_order().is_empty())
                .unwrap_or(false);
            if !has_layers {
                continue;
            }
            if *TRACE_CROSSMIN {
                eprintln!("crossmin: sweep graph {}", index);
            }

            match method {
                MinimizingMethod::NoCounter => self.minimize_crossings_no_counter(index),
                MinimizingMethod::CompareRandomized => {
                    self.compare_different_randomized_layouts(index)
                }
                MinimizingMethod::WithCounter => {
                    let _ = self.minimize_crossings_with_counter(index);
                }
            }

            if self
                .graph_info_holders
                .get(index)
                .map(|g| g.has_parent())
                .unwrap_or(false)
            {
                self.set_port_order_on_parent_graph(index);
            }
        }
    }

    fn minimize_crossings_no_counter(&mut self, index: usize) {
        let mut is_forward_sweep = self.next_boolean_for_graph(index);
        let mut improved = true;
        while improved {
            self.prepare_cross_minimizer(index);
            improved = {
                let graph_data = &mut self.graph_info_holders[index];
                graph_data.set_first_layer_order(is_forward_sweep)
            };
            improved |= self.sweep_reducing_crossings(index, is_forward_sweep, false);
            is_forward_sweep = !is_forward_sweep;
        }
        self.set_currently_best_node_orders();
    }

    fn compare_different_randomized_layouts(&mut self, index: usize) {
        let trace = *TRACE_CROSSMIN;
        self.reset_random_for_all_graphs();
        self.graphs_whose_node_order_changed.clear();

        let (node_influence, port_influence, consider_model_order_strategy, thoroughness) = {
            let graph_data = &self.graph_info_holders[index];
            (
                graph_data.node_influence(),
                graph_data.port_influence(),
                graph_data.consider_model_order_strategy(),
                graph_data.thoroughness(),
            )
        };
        if trace {
            eprintln!(
                "crossmin: compare randomized node_infl={} port_infl={} model_order_strategy={:?}",
                node_influence, port_influence, consider_model_order_strategy
            );
        }

        // Java has a copy-paste bug: checks NODE_INFLUENCE twice instead of NODE_INFLUENCE || PORT_INFLUENCE.
        // Match Java's behavior for parity.
        if node_influence != 0.0 {
            let mut best_crossings = f64::MAX;
            let consider_model_order = consider_model_order_strategy != OrderingStrategy::None;
            if consider_model_order {
                self.graph_info_holders[index].set_first_try_with_initial_order(true);
                if trace {
                    eprintln!("crossmin: compare randomized set first_try_with_initial_order=true");
                }
            }
            if trace {
                eprintln!("crossmin: compare randomized thoroughness={}", thoroughness);
            }
            self.graph_info_holders[index].reset_sweep_iteration();
            for _ in 0..thoroughness {
                if trace {
                    eprintln!("crossmin: compare randomized iter");
                }
                let crossings = self.minimize_crossings_node_port_order_with_counter(index);
                self.graph_info_holders[index].increment_sweep_iteration();
                if crossings < best_crossings {
                    best_crossings = crossings;
                    self.save_all_node_orders_of_changed_graphs();
                    if best_crossings == 0.0 {
                        break;
                    }
                }
            }
        } else {
            let mut best_crossings = i32::MAX;
            let consider_model_order = consider_model_order_strategy != OrderingStrategy::None;
            if consider_model_order {
                self.graph_info_holders[index].set_first_try_with_initial_order(true);
                if trace {
                    eprintln!("crossmin: compare randomized set first_try_with_initial_order=true");
                }
            }
            if trace {
                eprintln!("crossmin: compare randomized thoroughness={}", thoroughness);
            }
            self.graph_info_holders[index].reset_sweep_iteration();
            for _ in 0..thoroughness {
                if trace {
                    eprintln!("crossmin: compare randomized iter");
                }
                let crossings = self.minimize_crossings_with_counter(index);
                self.graph_info_holders[index].increment_sweep_iteration();
                if crossings < best_crossings {
                    best_crossings = crossings;
                    self.save_all_node_orders_of_changed_graphs();
                    if best_crossings == 0 {
                        break;
                    }
                }
            }
        }
        if trace {
            eprintln!("crossmin: compare randomized done");
        }
    }

    fn minimize_crossings_with_counter(&mut self, index: usize) -> i32 {
        let trace = *TRACE_CROSSMIN;
        let mut is_forward_sweep = self.next_boolean_for_graph(index);

        let initial_crossings = self.count_current_number_of_crossings(index) as f64;
        if trace {
            eprintln!(
                "crossmin: with_counter initial_crossings={}",
                initial_crossings
            );
        }
        let try_initial = self
            .graph_info_holders
            .get(index)
            .map(|g| g.first_try_with_initial_order())
            .unwrap_or(false);
        // Java: if (initialCrossings == 0 && FIRST_TRY_WITH_INITIAL_ORDER) { return 0; }
        if initial_crossings == 0.0 && try_initial {
            return 0;
        }

        let (first_try, second_try, model_order) = {
            let graph_data = &self.graph_info_holders[index];
            (
                graph_data.first_try_with_initial_order(),
                graph_data.second_try_with_initial_order(),
                graph_data.consider_model_order_strategy(),
            )
        };
        let use_initial = (first_try || second_try) && model_order != OrderingStrategy::None;
        if !use_initial {
            self.prepare_cross_minimizer(index);
            let graph_data = &mut self.graph_info_holders[index];
            graph_data.set_first_layer_order(is_forward_sweep);
        } else {
            is_forward_sweep = first_try;
        }

        self.sweep_reducing_crossings(index, is_forward_sweep, true);
        if trace {
            eprintln!("crossmin: with_counter after first sweep");
        }

        {
            let graph_data = &mut self.graph_info_holders[index];
            if graph_data.second_try_with_initial_order() {
                graph_data.set_second_try_with_initial_order(false);
            }
            if graph_data.first_try_with_initial_order() {
                graph_data.set_first_try_with_initial_order(false);
                graph_data.set_second_try_with_initial_order(true);
            }
        }

        let mut crossings_in_graph = self.count_current_number_of_crossings(index);
        if trace {
            eprintln!(
                "crossmin: with_counter crossings_in_graph={}",
                crossings_in_graph
            );
        }
        let mut old_number_of_crossings;
        loop {
            self.set_currently_best_node_orders();
            if crossings_in_graph == 0 {
                return 0;
            }
            is_forward_sweep = !is_forward_sweep;
            old_number_of_crossings = crossings_in_graph;
            self.sweep_reducing_crossings(index, is_forward_sweep, false);
            crossings_in_graph = self.count_current_number_of_crossings(index);
            if trace {
                eprintln!(
                    "crossmin: with_counter sweep done old={} new={}",
                    old_number_of_crossings, crossings_in_graph
                );
            }
            if old_number_of_crossings <= crossings_in_graph {
                break;
            }
        }
        old_number_of_crossings
    }

    fn minimize_crossings_node_port_order_with_counter(&mut self, index: usize) -> f64 {
        let trace = *TRACE_CROSSMIN;
        let mut is_forward_sweep = self.next_boolean_for_graph(index);

        let initial_crossings = self.count_current_number_of_crossings_node_port_order(index);
        if trace {
            eprintln!(
                "crossmin: node_port_with_counter initial_crossings={}",
                initial_crossings
            );
        }
        let try_initial = self
            .graph_info_holders
            .get(index)
            .map(|g| g.first_try_with_initial_order())
            .unwrap_or(false);
        // Java: if (initialCrossings == 0 && FIRST_TRY_WITH_INITIAL_ORDER) { return 0; }
        if initial_crossings == 0.0 && try_initial {
            return 0.0;
        }

        let (first_try, second_try, model_order) = {
            let graph_data = &self.graph_info_holders[index];
            (
                graph_data.first_try_with_initial_order(),
                graph_data.second_try_with_initial_order(),
                graph_data.consider_model_order_strategy(),
            )
        };
        let use_initial = (first_try || second_try) && model_order != OrderingStrategy::None;
        if !use_initial {
            self.prepare_cross_minimizer(index);
            let graph_data = &mut self.graph_info_holders[index];
            graph_data.set_first_layer_order(is_forward_sweep);
        } else {
            is_forward_sweep = first_try;
        }

        self.sweep_reducing_crossings(index, is_forward_sweep, true);
        if trace {
            eprintln!("crossmin: node_port_with_counter after first sweep");
        }

        {
            let graph_data = &mut self.graph_info_holders[index];
            if graph_data.second_try_with_initial_order() {
                graph_data.set_second_try_with_initial_order(false);
            }
            if graph_data.first_try_with_initial_order() {
                graph_data.set_first_try_with_initial_order(false);
                graph_data.set_second_try_with_initial_order(true);
            }
        }

        let mut crossings_in_graph = self.count_current_number_of_crossings_node_port_order(index);
        if trace {
            eprintln!(
                "crossmin: node_port_with_counter crossings_in_graph={}",
                crossings_in_graph
            );
        }
        let mut old_number_of_crossings;
        loop {
            self.set_currently_best_node_orders();
            if crossings_in_graph == 0.0 {
                return 0.0;
            }
            is_forward_sweep = !is_forward_sweep;
            old_number_of_crossings = crossings_in_graph;
            self.sweep_reducing_crossings(index, is_forward_sweep, false);
            crossings_in_graph = self.count_current_number_of_crossings_node_port_order(index);
            if trace {
                eprintln!(
                    "crossmin: node_port_with_counter sweep done old={} new={}",
                    old_number_of_crossings, crossings_in_graph
                );
            }
            if old_number_of_crossings <= crossings_in_graph {
                break;
            }
        }
        old_number_of_crossings
    }

    fn count_model_order_node_changes(
        &self,
        graph: &LGraphRef,
        layers: &[Vec<LNodeRef>],
        strategy: OrderingStrategy,
        group_strategy: GroupOrderStrategy,
    ) -> i32 {
        let mut previous_layer: Option<Vec<LNodeRef>> = None;
        let mut wrong_model_order = 0;
        for layer in layers {
            let prev_layer = previous_layer
                .clone()
                .unwrap_or_else(|| layers.first().cloned().unwrap_or_default());
            let mut comp = ModelOrderNodeComparator::new(
                graph.clone(),
                prev_layer,
                strategy,
                LongEdgeOrderingStrategy::Equal,
                group_strategy,
                false,
            );
            for i in 0..layer.len() {
                for j in (i + 1)..layer.len() {
                    let has_i = layer[i]
                        .lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::MODEL_ORDER)
                        })
                        .is_some();
                    let has_j = layer[j]
                        .lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::MODEL_ORDER)
                        })
                        .is_some();
                    if has_i && has_j && comp.compare(&layer[i], &layer[j]) > 0 {
                        wrong_model_order += 1;
                    }
                }
            }
            previous_layer = Some(layer.clone());
        }
        wrong_model_order
    }

    fn count_model_order_port_changes(
        &self,
        graph: &LGraphRef,
        layers: &[Vec<LNodeRef>],
        strategy: OrderingStrategy,
        port_model_order: bool,
        group_strategy: GroupOrderStrategy,
    ) -> i32 {
        let mut previous_layer: Option<Vec<LNodeRef>> = None;
        let mut wrong_model_order = 0;
        for layer in layers {
            let prev_layer = previous_layer
                .clone()
                .unwrap_or_else(|| layers.first().cloned().unwrap_or_default());
            for node in layer {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                let long_edge_targets =
                    SortByInputModelProcessor::long_edge_target_node_preprocessing(node);
                let mut comp = ModelOrderPortComparator::new(
                    graph.clone(),
                    prev_layer.clone(),
                    strategy,
                    Some(long_edge_targets),
                    port_model_order,
                );
                for i in 0..ports.len() {
                    for j in (i + 1)..ports.len() {
                        if comp.compare(&ports[i], &ports[j]) > 0 {
                            wrong_model_order += 1;
                        }
                    }
                }
                comp.clear_transitive_ordering();
                let _ = group_strategy;
            }
            previous_layer = Some(layer.clone());
        }
        wrong_model_order
    }

    fn count_current_number_of_crossings(&mut self, start_index: usize) -> i32 {
        let trace = *TRACE_CROSSMIN_TIMING;
        let start = if trace {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let mut total_crossings = 0;
        let mut stack = VecDeque::new();
        stack.push_back(start_index);
        while let Some(index) = stack.pop_back() {
            let crossings = {
                let graph_data = &mut self.graph_info_holders[index];
                graph_data.count_current_crossings()
            };
            total_crossings += crossings;
            let child_indices = self.child_graph_indices(index);
            for child_index in child_indices {
                if !self.graph_info_holders[child_index].dont_sweep_into() {
                    stack.push_back(child_index);
                }
            }
        }
        if let Some(start) = start {
            eprintln!(
                "crossmin: count_crossings index={} total={} took {} ms",
                start_index,
                total_crossings,
                start.elapsed().as_millis()
            );
        }
        total_crossings
    }

    fn count_current_number_of_crossings_node_port_order(&mut self, start_index: usize) -> f64 {
        let trace = *TRACE_CROSSMIN_TIMING;
        let start = if trace {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let mut total_crossings = 0.0;
        // Match Java's implementation details for parity:
        // - Use the root graph's model-order settings for all counted layers.
        // - Recurse into children with plain crossing count (without model-order influence).
        let (
            root_graph_ref,
            root_model_order_strategy,
            root_group_strategy,
            root_port_model_order,
            root_node_influence,
            root_port_influence,
        ) = {
            let graph_data = &self.graph_info_holders[start_index];
            (
                graph_data.l_graph().clone(),
                graph_data.consider_model_order_strategy(),
                graph_data.group_order_strategy(),
                graph_data.port_model_order(),
                graph_data.node_influence(),
                graph_data.port_influence(),
            )
        };
        let mut stack = VecDeque::new();
        stack.push_back(start_index);
        while let Some(index) = stack.pop_back() {
            let crossings = {
                let graph_data = &mut self.graph_info_holders[index];
                graph_data.count_current_crossings() as f64
            };
            let mut model_order_influence = 0.0;
            if root_model_order_strategy != OrderingStrategy::None {
                let order = self.graph_info_holders[index].current_node_order();
                model_order_influence += root_node_influence
                    * self.count_model_order_node_changes(
                        &root_graph_ref,
                        order,
                        root_model_order_strategy,
                        root_group_strategy,
                    ) as f64;
                model_order_influence += root_port_influence
                    * self.count_model_order_port_changes(
                        &root_graph_ref,
                        order,
                        root_model_order_strategy,
                        root_port_model_order,
                        root_group_strategy,
                    ) as f64;
            }
            total_crossings += crossings + model_order_influence;
            let child_indices = self.child_graph_indices(index);
            for child_index in child_indices {
                if !self.graph_info_holders[child_index].dont_sweep_into() {
                    total_crossings += self.count_current_number_of_crossings(child_index) as f64;
                }
            }
        }
        if let Some(start) = start {
            eprintln!(
                "crossmin: count_crossings_node_port index={} total={} took {} ms",
                start_index,
                total_crossings,
                start.elapsed().as_millis()
            );
        }
        total_crossings
    }

    fn sweep_reducing_crossings(&mut self, index: usize, forward: bool, first_sweep: bool) -> bool {
        let trace = *TRACE_CROSSMIN;
        let length = self.graph_info_holders[index].current_node_order().len();
        if length == 0 {
            return false;
        }
        self.prepare_cross_minimizer(index);
        let timing = *TRACE_CROSSMIN_TIMING;
        let mut improved = {
            let graph_data = &mut self.graph_info_holders[index];
            if trace {
                eprintln!(
                    "crossmin: distribute_ports layer={} forward={} first={}",
                    first_index(forward, length),
                    forward,
                    first_sweep
                );
            }
            let start = if timing {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let improved =
                graph_data.distribute_ports_while_sweeping(first_index(forward, length), forward);
            if let Some(start) = start {
                eprintln!(
                    "crossmin: distribute_ports layer={} done in {} ms",
                    first_index(forward, length),
                    start.elapsed().as_millis()
                );
            }
            improved
        };
        if trace {
            let order = self.graph_info_holders[index].current_node_order();
            let first = first_index(forward, length);
            eprintln!(
                "crossmin: sweep layer={} order=[{}]",
                first,
                format_layer_nodes(&order[first])
            );
        }
        let first_layer = self.graph_info_holders[index].current_node_order()
            [first_index(forward, length)]
        .clone();
        let start = if timing {
            Some(std::time::Instant::now())
        } else {
            None
        };
        improved |= self.sweep_in_hierarchical_nodes(&first_layer, forward, first_sweep);
        if let Some(start) = start {
            eprintln!(
                "crossmin: sweep_in_hierarchical_nodes layer={} done in {} ms",
                first_index(forward, length),
                start.elapsed().as_millis()
            );
        }

        let mut i = first_free(forward, length);
        while is_not_end(length, i, forward) {
            let i_usize = i as usize;
            let allow_first_sweep = {
                let graph_data = &self.graph_info_holders[index];
                let first_try = graph_data.first_try_with_initial_order();
                let second_try = graph_data.second_try_with_initial_order();
                first_sweep && !first_try && !second_try
            };

            self.prepare_cross_minimizer(index);
            {
                let graph_data = &mut self.graph_info_holders[index];
                if trace {
                    eprintln!(
                        "crossmin: minimize layer={} forward={} allow_first={}",
                        i_usize, forward, allow_first_sweep
                    );
                }
                improved |=
                    graph_data.minimize_crossings_on_layer(i_usize, forward, allow_first_sweep);
                if trace {
                    eprintln!(
                        "crossmin: distribute_ports layer={} forward={} first={}",
                        i_usize, forward, first_sweep
                    );
                }
                let start = if timing {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                let distributed = graph_data.distribute_ports_while_sweeping(i_usize, forward);
                if let Some(start) = start {
                    eprintln!(
                        "crossmin: distribute_ports layer={} done in {} ms",
                        i_usize,
                        start.elapsed().as_millis()
                    );
                }
                improved |= distributed;
            }
            if trace {
                let order = self.graph_info_holders[index].current_node_order();
                eprintln!(
                    "crossmin: sweep layer={} order=[{}]",
                    i_usize,
                    format_layer_nodes(&order[i_usize])
                );
            }
            let layer = self.graph_info_holders[index].current_node_order()[i_usize].clone();
            let start = if timing {
                Some(std::time::Instant::now())
            } else {
                None
            };
            improved |= self.sweep_in_hierarchical_nodes(&layer, forward, first_sweep);
            if let Some(start) = start {
                eprintln!(
                    "crossmin: sweep_in_hierarchical_nodes layer={} done in {} ms",
                    i_usize,
                    start.elapsed().as_millis()
                );
            }
            i += next(forward);
        }

        self.graphs_whose_node_order_changed.insert(index);
        improved
    }

    fn sweep_in_hierarchical_nodes(
        &mut self,
        layer: &[LNodeRef],
        is_forward_sweep: bool,
        is_first_sweep: bool,
    ) -> bool {
        let mut improved = false;
        for node in layer {
            let nested_graph = node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.nested_graph());
            if let Some(nested_graph) = nested_graph {
                let nested_index = graph_id(&nested_graph);
                if let Some(nested_index) = nested_index {
                    if !self.graph_info_holders[nested_index].dont_sweep_into() {
                        improved |= self.sweep_in_hierarchical_node(
                            is_forward_sweep,
                            node,
                            nested_index,
                            is_first_sweep,
                        );
                    }
                }
            }
        }
        improved
    }

    fn sweep_in_hierarchical_node(
        &mut self,
        is_forward_sweep: bool,
        node: &LNodeRef,
        nested_index: usize,
        is_first_sweep: bool,
    ) -> bool {
        let start_index = {
            let order = self.graph_info_holders[nested_index].current_node_order();
            first_index(is_forward_sweep, order.len())
        };
        let first_node = self.graph_info_holders[nested_index].current_node_order()[start_index]
            .first()
            .cloned();

        if let Some(first_node) = first_node {
            if is_external_port_dummy(&first_node) {
                let sorted = {
                    let order = self.graph_info_holders[nested_index].current_node_order();
                    let layer = &order[start_index];
                    sort_port_dummies_by_port_positions(
                        node,
                        layer,
                        side_opposed_sweep_direction(is_forward_sweep),
                    )
                };
                self.graph_info_holders[nested_index].current_node_order_mut()[start_index] =
                    sorted;
            } else {
                self.prepare_cross_minimizer(nested_index);
                let graph_data = &mut self.graph_info_holders[nested_index];
                graph_data.set_first_layer_order(is_forward_sweep);
            }
        }

        let improved =
            self.sweep_reducing_crossings(nested_index, is_forward_sweep, is_first_sweep);
        if let Some(parent) = self.graph_info_holders[nested_index].parent() {
            sort_ports_by_dummy_positions_in_last_layer(
                self.graph_info_holders[nested_index].current_node_order(),
                &parent,
                is_forward_sweep,
            );
        }

        improved
    }

    fn set_port_order_on_parent_graph(&mut self, index: usize) {
        let has_external_ports = self.graph_info_holders[index].has_external_ports();
        let best_sweep = self.graph_info_holders[index]
            .best_sweep()
            .map(|s| s.nodes());
        if !has_external_ports || best_sweep.is_none() {
            return;
        }
        let best_sweep = best_sweep.unwrap();
        let Some(parent) = self.graph_info_holders[index].parent() else {
            return;
        };

        sort_ports_by_dummy_positions_in_last_layer(best_sweep, &parent, true);
        sort_ports_by_dummy_positions_in_last_layer(best_sweep, &parent, false);
        if let Ok(mut parent_guard) = parent.lock() {
            parent_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedOrder),
            );
        };
    }

    fn save_all_node_orders_of_changed_graphs(&mut self) {
        let indices: Vec<usize> = self
            .graphs_whose_node_order_changed
            .iter()
            .copied()
            .collect();
        for index in indices {
            self.graph_info_holders[index].update_best_from_currently_best_or_current();
        }
    }

    fn set_currently_best_node_orders(&mut self) {
        let indices: Vec<usize> = self
            .graphs_whose_node_order_changed
            .iter()
            .copied()
            .collect();
        for index in indices {
            self.graph_info_holders[index].update_currently_best_from_current();
        }
    }

    fn prepare_cross_minimizer(&mut self, index: usize) {
        // Parent snapshot cloning is only needed for greedy-switch variants.
        if !matches!(
            self.cross_min_type,
            CrossMinType::OneSidedGreedySwitch | CrossMinType::TwoSidedGreedySwitch
        ) {
            return;
        }
        let has_parent = self
            .graph_info_holders
            .get(index)
            .map(|holder| holder.has_parent())
            .unwrap_or(false);
        let parent_snapshot = if has_parent {
            self.parent_snapshot_for(index)
        } else {
            None
        };
        if let Some(graph_data) = self.graph_info_holders.get_mut(index) {
            graph_data.update_greedy_context(parent_snapshot);
        }
    }

    fn parent_snapshot_for(
        &self,
        index: usize,
    ) -> Option<(Vec<Vec<LNodeRef>>, Vec<i32>, LNodeRef)> {
        let parent_index = self.graph_info_holders.get(index)?.parent_graph_index()?;
        let parent = self.graph_info_holders.get(parent_index)?;
        let parent_node = self.graph_info_holders.get(index)?.parent()?;
        Some((
            parent.current_node_order().clone(),
            parent.port_positions().clone(),
            parent_node,
        ))
    }

    fn child_graph_indices(&self, index: usize) -> Vec<usize> {
        let graph_data = &self.graph_info_holders[index];
        graph_data
            .child_graphs()
            .iter()
            .filter_map(graph_id)
            .collect()
    }

    #[allow(dead_code)] // Java parity: used in Java but not yet wired in Rust
    fn has_single_external_port_boundaries(&self, index: usize) -> bool {
        let Some(order) = self
            .graph_info_holders
            .get(index)
            .map(|holder| holder.current_node_order())
        else {
            return false;
        };
        if order.len() < 5 {
            return false;
        }
        let (Some(first_layer), Some(last_layer)) = (order.first(), order.last()) else {
            return false;
        };
        if first_layer.len() != 1 || last_layer.len() != 1 {
            return false;
        }
        is_external_port_dummy(&first_layer[0]) && is_external_port_dummy(&last_layer[0])
    }

    fn reset_random_for_all_graphs(&mut self) {
        if *TRACE_CROSSMIN {
            eprintln!("crossmin: reset_random_for_all_graphs");
        }
        // Java only resets the shared random object, NOT individual heuristic seeds
        self.random.set_seed(self.random_seed);
    }

    fn next_boolean_for_graph(&mut self, _index: usize) -> bool {
        if let Some(force) = DEBUG_CROSSMIN_FORCE_SWEEP.as_deref() {
            match force {
                "forward" => return true,
                "backward" => return false,
                _ => {}
            }
        }
        // Java uses the shared LayerSweepCrossingMinimizer random directly for sweep direction.
        let v = self.random.next_boolean();
        random_trace::trace_next_boolean(v, "crossing_min::sweep_direction")
    }

    fn initialize(&mut self, root_graph: &LGraphRef, root_graph_guard: &mut LGraph) -> Vec<usize> {
        self.graph_info_holders.clear();
        self.graphs_whose_node_order_changed.clear();
        let trace = *TRACE_CROSSMIN;

        self.random = root_graph_guard
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_default();
        // Reset the random-trace counter at the start of each new graph layout.
        random_trace::reset_counter();
        let raw_seed = self.random.next_long();
        self.random_seed =
            random_trace::trace_next_long(raw_seed, "graph_info_holder::initialize") as u64;
        if trace {
            eprintln!(
                "crossmin:init root_ptr={:p} random_seed={}",
                Arc::as_ptr(root_graph),
                self.random_seed as i64
            );
            eprintln!(
                "[CROSSMIN] initialize: random_seed={} (i64={})",
                self.random_seed,
                self.random_seed as i64
            );
        }

        let mut graphs_to_sweep_on: Vec<usize> = Vec::new();
        let mut queue: VecDeque<LGraphRef> = VecDeque::new();
        queue.push_back(root_graph.clone());

        while let Some(graph) = queue.pop_front() {
            let index = self.graph_info_holders.len();
            if trace {
                eprintln!("crossmin:init graph {}", index);
            }
            if Arc::ptr_eq(&graph, root_graph) {
                root_graph_guard.graph_element().id = index as i32;
            } else if let Ok(mut graph_guard) = graph.lock() {
                graph_guard.graph_element().id = index as i32;
            }
            let mut g_data = if Arc::ptr_eq(&graph, root_graph) {
                GraphInfoHolder::new_with_graph(
                    graph.clone(),
                    root_graph_guard,
                    self.cross_min_type,
                    &mut self.random,
                )
            } else {
                GraphInfoHolder::new(graph.clone(), self.cross_min_type)
            };
            if *TRACE_CROSSMIN_STATS {
                Self::log_graph_stats(index, &g_data);
            }
            let parent_index = g_data.parent_graph_ref().and_then(|parent_graph| {
                self.graph_info_holders
                    .iter()
                    .position(|holder| Arc::ptr_eq(holder.l_graph(), parent_graph))
            });
            g_data.set_parent_graph_index(parent_index);
            if trace {
                eprintln!(
                    "crossmin:init graph {} child_graphs={}",
                    index,
                    g_data.child_graphs().len()
                );
            }
            for child_graph in g_data.child_graphs().iter().cloned() {
                queue.push_back(child_graph);
            }
            self.graph_info_holders.push(g_data);
            if self.graph_info_holders[index].dont_sweep_into() {
                graphs_to_sweep_on.insert(0, index);
            }
        }

        graphs_to_sweep_on
    }

    fn log_graph_stats(index: usize, holder: &GraphInfoHolder) {
        let layers = holder.current_node_order();
        let layer_count = layers.len();
        let node_count: usize = layers.iter().map(|layer| layer.len()).sum();
        let mut port_count = 0usize;
        let mut edges: HashSet<usize> = HashSet::new();

        for layer in layers {
            for node in layer {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                port_count += ports.len();
                for port in ports {
                    let connected = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.connected_edges().clone())
                        .unwrap_or_default();
                    for edge in connected {
                        edges.insert(Arc::as_ptr(&edge) as usize);
                    }
                }
            }
        }

        eprintln!(
            "crossmin: graph_stats index={} layers={} nodes={} ports={} edges={}",
            index,
            layer_count,
            node_count,
            port_count,
            edges.len()
        );
    }

    fn transfer_node_and_port_orders_to_graph(
        &mut self,
        root_graph: &LGraphRef,
        root_graph_guard: &mut LGraph,
    ) {
        for graph_data in &self.graph_info_holders {
            if let Some(best_sweep) = graph_data.best_sweep() {
                if Arc::ptr_eq(graph_data.l_graph(), root_graph) {
                    best_sweep.transfer_node_and_port_orders_to_graph_guard(root_graph_guard, true);
                } else {
                    best_sweep.transfer_node_and_port_orders_to_graph(graph_data.l_graph(), true);
                }
            }
        }
    }
}

impl Default for LayerSweepCrossingMinimizer {
    fn default() -> Self {
        LayerSweepCrossingMinimizer::new(CrossMinType::Barycenter)
    }
}

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::LongEdgeSplitter),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::InLayerConstraintProcessor),
        )
        .after(LayeredPhases::P5EdgeRouting)
        .add(Arc::new(IntermediateProcessorStrategy::LongEdgeJoiner));
    config
});

impl ILayoutPhase<LayeredPhases, LGraph> for LayerSweepCrossingMinimizer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin(
            &format!("Minimize Crossings {:?}", self.cross_min_type),
            1.0,
        );
        let trace = *TRACE_CROSSMIN;
        if trace {
            eprintln!("crossmin: start");
        }
        if !ElkReflect::has_clone::<Vec<LNodeRef>>() {
            ElkReflect::register_default_clone::<Vec<LNodeRef>>();
        }

        let empty_graph = graph.layers().is_empty()
            || graph.layers().iter().all(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().is_empty())
                    .unwrap_or(true)
            });
        if trace {
            eprintln!("crossmin: empty_graph={}", empty_graph);
        }
        let single_node = graph.layers().len() == 1
            && graph
                .layers()
                .first()
                .and_then(|layer| layer.lock().ok())
                .map(|layer_guard| layer_guard.nodes().len())
                .unwrap_or(0)
                == 1;
        if trace {
            eprintln!("crossmin: single_node={}", single_node);
        }
        let hierarchical_layout = graph
            .get_property(LayeredOptions::HIERARCHY_HANDLING)
            .unwrap_or(HierarchyHandling::Inherit)
            == HierarchyHandling::IncludeChildren;
        if trace {
            eprintln!("crossmin: hierarchical_layout={}", hierarchical_layout);
        }

        if empty_graph || (single_node && !hierarchical_layout) {
            monitor.done();
            return;
        }

        let root_graph = match root_graph_ref(graph) {
            Some(graph_ref) => graph_ref,
            None => {
                monitor.done();
                return;
            }
        };

        let graphs_to_sweep_on = self.initialize(&root_graph, graph);
        if graphs_to_sweep_on.is_empty() {
            monitor.done();
            return;
        }
        if trace {
            eprintln!(
                "crossmin: graphs={}, sweep_targets={:?}",
                self.graph_info_holders.len(),
                graphs_to_sweep_on
            );
            // Trace 1: layer structure at start of process()
            for (gi, holder) in self.graph_info_holders.iter().enumerate() {
                let order = holder.current_node_order();
                eprintln!("[CROSSMIN] process: graph={} layers={}", gi, order.len());
                for (li, layer) in order.iter().enumerate() {
                    let node_ids: Vec<String> = layer
                        .iter()
                        .map(|n| {
                            n.lock()
                                .ok()
                                .map(|mut g| {
                                    format!("id:{}", g.shape().graph_element().id)
                                })
                                .unwrap_or_else(|| "<locked>".to_string())
                        })
                        .collect();
                    eprintln!(
                        "[CROSSMIN] process: graph={} layer={} nodes={} ids=[{}]",
                        gi,
                        li,
                        layer.len(),
                        node_ids.join(", ")
                    );
                }
            }
        }
        let method = self.choose_minimizing_method(graphs_to_sweep_on[0]);
        if trace {
            eprintln!("crossmin: method={:?}", method);
        }
        self.minimize_crossings(&graphs_to_sweep_on, method);
        if trace {
            eprintln!("crossmin: minimize_crossings done");
        }
        self.transfer_node_and_port_orders_to_graph(&root_graph, graph);
        if trace {
            eprintln!("crossmin: transfer done");
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let mut config =
            LayoutProcessorConfiguration::create_from(&INTERMEDIATE_PROCESSING_CONFIGURATION);
        config.add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::PortListSorter),
        );
        Some(config)
    }

    fn is_hierarchy_aware(&self) -> bool {
        true
    }
}

fn first_index(is_forward: bool, length: usize) -> usize {
    if is_forward {
        0
    } else {
        length.saturating_sub(1)
    }
}

fn end_index(is_forward: bool, length: usize) -> usize {
    if is_forward {
        length.saturating_sub(1)
    } else {
        0
    }
}

fn first_free(is_forward: bool, length: usize) -> isize {
    if is_forward {
        1
    } else {
        length as isize - 2
    }
}

fn next(is_forward: bool) -> isize {
    if is_forward {
        1
    } else {
        -1
    }
}

fn is_not_end(length: usize, index: isize, is_forward: bool) -> bool {
    if is_forward {
        index < length as isize
    } else {
        index >= 0
    }
}

fn side_opposed_sweep_direction(is_forward: bool) -> PortSide {
    if is_forward {
        PortSide::West
    } else {
        PortSide::East
    }
}

fn is_external_port_dummy(node: &LNodeRef) -> bool {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.node_type() == NodeType::ExternalPort)
        .unwrap_or(false)
}

fn is_hierarchical(port: &LPortRef) -> bool {
    port.lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::INSIDE_CONNECTIONS))
        .unwrap_or(false)
}

fn origin_port(node: &LNodeRef) -> Option<LPortRef> {
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            crate::org::eclipse::elk::alg::layered::options::Origin::LPort(port) => Some(port),
            _ => None,
        })
}

fn dummy_node_for(port: &LPortRef) -> Option<LNodeRef> {
    port.lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY))
}

fn is_on_end_of_sweep_side(port: &LPortRef, on_right_most_layer: bool) -> bool {
    let side = port
        .lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);
    if on_right_most_layer {
        side == PortSide::East
    } else {
        side == PortSide::West
    }
}

fn sort_ports_by_dummy_positions_in_last_layer(
    node_order: &[Vec<LNodeRef>],
    parent: &LNodeRef,
    on_right_most_layer: bool,
) {
    let end_index = end_index(on_right_most_layer, node_order.len());
    let last_layer = match node_order.get(end_index) {
        Some(layer) => layer,
        None => return,
    };
    let mut j = first_index(on_right_most_layer, last_layer.len());
    if last_layer.is_empty() || !is_external_port_dummy(&last_layer[j]) {
        return;
    }

    if let Ok(mut parent_guard) = parent.lock() {
        let ports = parent_guard.ports_mut();
        for i in 0..ports.len() {
            let port = ports.get(i).cloned();
            let Some(port) = port else {
                continue;
            };
            if is_on_end_of_sweep_side(&port, on_right_most_layer) && is_hierarchical(&port) {
                if let Some(origin) = origin_port(&last_layer[j]) {
                    ports[i] = origin;
                    j = ((j as isize) + next(on_right_most_layer)) as usize;
                }
            }
        }
    }
}

fn sort_port_dummies_by_port_positions(
    parent_node: &LNodeRef,
    layer_close_to_edge: &[LNodeRef],
    side: PortSide,
) -> Vec<LNodeRef> {
    let ports = in_north_south_east_west_order(parent_node, side);
    let mut sorted_dummies: Vec<LNodeRef> = Vec::with_capacity(layer_close_to_edge.len());
    for port in ports {
        if is_hierarchical(&port) {
            if let Some(dummy) = dummy_node_for(&port) {
                sorted_dummies.push(dummy);
            }
        }
    }
    if sorted_dummies.len() < layer_close_to_edge.len() {
        panic!(
            "Expected {} hierarchical ports, but found only {}.",
            layer_close_to_edge.len(),
            sorted_dummies.len()
        );
    }
    sorted_dummies
}

fn graph_id(graph: &LGraphRef) -> Option<usize> {
    graph
        .lock()
        .ok()
        .map(|mut graph_guard| graph_guard.graph_element().id as usize)
}

fn root_graph_ref(graph: &mut LGraph) -> Option<LGraphRef> {
    if let Some(layer) = graph.layers().first() {
        if let Ok(layer_guard) = layer.lock() {
            return layer_guard.graph();
        }
    }
    if let Some(node) = graph.layerless_nodes().first() {
        if let Ok(node_guard) = node.lock() {
            return node_guard.graph();
        }
    }
    None
}

fn format_layer_nodes(layer: &[LNodeRef]) -> String {
    layer
        .iter()
        .map(|node| {
            node.lock()
                .ok()
                .map(|mut guard| guard.to_string())
                .unwrap_or_else(|| String::from("<poisoned-node>"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}
