use std::sync::{Arc, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::greedy_switch_heuristic::GreedySwitchHeuristic;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, GroupOrderStrategy, InternalProperties, LayeredOptions, OrderingStrategy,
};
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_heuristic::BarycenterHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_port_distributor::BarycenterPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::counting::{
    init_initializables, AllCrossingsCounter, IInitializable,
};
use crate::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;
use crate::org::eclipse::elk::alg::layered::p3order::layer_sweep_type_decider::LayerSweepTypeDecider;
use crate::org::eclipse::elk::alg::layered::p3order::median_heuristic::MedianHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::model_order_barycenter_heuristic::ModelOrderBarycenterHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::{
    GreedyPortDistributor, LayerTotalPortDistributor, NodeRelativePortDistributor,
};
use crate::org::eclipse::elk::alg::layered::p3order::sweep_copy::SweepCopy;

pub struct GraphInfoHolder {
    l_graph: LGraphRef,
    current_node_order: Vec<Vec<LNodeRef>>,
    currently_best_node_and_port_order: Option<SweepCopy>,
    best_node_and_port_order: Option<SweepCopy>,
    port_positions: Vec<i32>,
    use_bottom_up: bool,
    child_graphs: Vec<LGraphRef>,
    has_external_ports: bool,
    has_parent: bool,
    parent_graph_ref: Option<LGraphRef>,
    parent_graph_index: Option<usize>,
    parent_node: Option<LNodeRef>,
    cross_minimizer: Box<dyn ICrossingMinimizationHeuristic>,
    port_distributor: Box<dyn ISweepPortDistributor>,
    crossings_counter: AllCrossingsCounter,
    n_ports: usize,
    consider_model_order_strategy: OrderingStrategy,
    group_order_strategy: GroupOrderStrategy,
    port_model_order: bool,
    node_influence: f64,
    port_influence: f64,
    thoroughness: i32,
    first_try_with_initial_order: bool,
    second_try_with_initial_order: bool,
}

impl GraphInfoHolder {
    pub fn new(
        graph: LGraphRef,
        cross_min_type: CrossMinType,
    ) -> Self {
        let current_node_order = graph
            .lock()
            .ok()
            .map(|graph_guard| graph_guard.to_node_array())
            .unwrap_or_default();
        let parent_node = graph.lock().ok().and_then(|graph_guard| graph_guard.parent_node());
        let graph_properties = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::GRAPH_PROPERTIES))
            .unwrap_or_default();
        let random = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::RANDOM))
            .unwrap_or_default();
        let force_model_order = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER)
            })
            .unwrap_or(false);
        let max_model_order_nodes = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::MAX_MODEL_ORDER_NODES))
            .unwrap_or(0);
        let constraints_between_non_dummies = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(
                    InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES,
                )
            })
            .unwrap_or(false);
        let hierarchical_sweepiness = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS)
            })
            .unwrap_or(0.0);
        let consider_model_order_strategy = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
            })
            .unwrap_or(OrderingStrategy::None);
        let group_order_strategy = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
            })
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup);
        let port_model_order = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER)
            })
            .unwrap_or(false);
        let node_influence = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE)
            })
            .unwrap_or(0.0);
        let port_influence = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE)
            })
            .unwrap_or(0.0);
        let thoroughness = graph
            .lock()
            .ok()
            .and_then(|mut graph_guard| graph_guard.get_property(LayeredOptions::THOROUGHNESS))
            .unwrap_or(1);

        Self::build(
            graph,
            cross_min_type,
            current_node_order,
            parent_node,
            graph_properties,
            random,
            force_model_order,
            max_model_order_nodes,
            constraints_between_non_dummies,
            hierarchical_sweepiness,
            consider_model_order_strategy,
            group_order_strategy,
            port_model_order,
            node_influence,
            port_influence,
            thoroughness,
        )
    }

    pub fn new_with_graph(
        graph_ref: LGraphRef,
        graph: &mut LGraph,
        cross_min_type: CrossMinType,
    ) -> Self {
        let trace = std::env::var_os("ELK_TRACE_CROSSMIN").is_some();
        if trace {
            eprintln!("crossmin: graph_info_holder new_with_graph start");
        }
        let current_node_order = graph.to_node_array();
        if trace {
            eprintln!("crossmin: graph_info_holder new_with_graph node_order");
        }
        let parent_node = graph.parent_node();
        let graph_properties = graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_default();
        if trace {
            eprintln!("crossmin: graph_info_holder new_with_graph properties");
        }
        let random = graph
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_default();
        let force_model_order = graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER)
            .unwrap_or(false);
        let max_model_order_nodes = graph
            .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
            .unwrap_or(0);
        let constraints_between_non_dummies = graph
            .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES)
            .unwrap_or(false);
        let hierarchical_sweepiness = graph
            .get_property(LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS)
            .unwrap_or(0.0);
        let consider_model_order_strategy = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
            .unwrap_or(OrderingStrategy::None);
        let group_order_strategy = graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup);
        let port_model_order = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER)
            .unwrap_or(false);
        let node_influence = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE)
            .unwrap_or(0.0);
        let port_influence = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE)
            .unwrap_or(0.0);
        let thoroughness = graph
            .get_property(LayeredOptions::THOROUGHNESS)
            .unwrap_or(1);

        Self::build(
            graph_ref,
            cross_min_type,
            current_node_order,
            parent_node,
            graph_properties,
            random,
            force_model_order,
            max_model_order_nodes,
            constraints_between_non_dummies,
            hierarchical_sweepiness,
            consider_model_order_strategy,
            group_order_strategy,
            port_model_order,
            node_influence,
            port_influence,
            thoroughness,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        graph: LGraphRef,
        cross_min_type: CrossMinType,
        current_node_order: Vec<Vec<LNodeRef>>,
        parent_node: Option<LNodeRef>,
        graph_properties: EnumSet<GraphProperties>,
        random: Random,
        force_model_order: bool,
        max_model_order_nodes: i32,
        constraints_between_non_dummies: bool,
        hierarchical_sweepiness: f64,
        consider_model_order_strategy: OrderingStrategy,
        group_order_strategy: GroupOrderStrategy,
        port_model_order: bool,
        node_influence: f64,
        port_influence: f64,
        thoroughness: i32,
    ) -> Self {
        let trace = std::env::var_os("ELK_TRACE_CROSSMIN").is_some();
        if trace {
            eprintln!("crossmin: graph_info_holder build start");
        }
        let mut random = random;
        trace_in_layer_constraints(&current_node_order);
        let has_parent = parent_node.is_some();
        let parent_graph_ref = parent_node
            .as_ref()
            .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()));
        // Parent graph indices are resolved by the caller once graph holders are collected.
        let parent_graph_index = None;

        let has_external_ports = graph_properties.contains(&GraphProperties::ExternalPorts);

        let (port_distributor, mut barycenter_port_distributor) =
            create_port_distributors(cross_min_type, &mut random, current_node_order.len());
        if trace {
            eprintln!("crossmin: graph_info_holder port distributors ready");
        }

        let crossings_counter = AllCrossingsCounter::new(&current_node_order);
        if trace {
            eprintln!("crossmin: graph_info_holder crossings counter ready");
        }

        let cross_minimizer: Box<dyn ICrossingMinimizationHeuristic> = match cross_min_type {
            CrossMinType::Barycenter => {
                let resolver =
                    ForsterConstraintResolver::new(&current_node_order, constraints_between_non_dummies);
                let distributor = barycenter_port_distributor
                    .take()
                    .expect("barycenter distributor missing");
                if force_model_order {
                    Box::new(ModelOrderBarycenterHeuristic::new(
                        resolver,
                        random,
                        distributor,
                        force_model_order,
                        max_model_order_nodes,
                        group_order_strategy,
                    ))
                } else {
                    Box::new(BarycenterHeuristic::new(resolver, random, distributor))
                }
            }
            CrossMinType::Median => Box::new(MedianHeuristic::new(random)),
            CrossMinType::OneSidedGreedySwitch | CrossMinType::TwoSidedGreedySwitch => {
                Box::new(GreedySwitchHeuristic::new(cross_min_type, has_parent))
            }
        };
        if trace {
            eprintln!("crossmin: graph_info_holder cross minimizer ready");
        }

        let cross_min_deterministic = cross_minimizer.is_deterministic();
        let mut layer_sweep_type_decider =
            LayerSweepTypeDecider::new(graph.clone(), parent_node.clone(), has_parent, cross_min_deterministic);
        if trace {
            eprintln!("crossmin: graph_info_holder sweep type decider ready");
        }

        let mut holder = GraphInfoHolder {
            l_graph: graph,
            current_node_order,
            currently_best_node_and_port_order: None,
            best_node_and_port_order: None,
            port_positions: Vec::new(),
            use_bottom_up: false,
            child_graphs: Vec::new(),
            has_external_ports,
            has_parent,
            parent_graph_ref,
            parent_graph_index,
            parent_node,
            cross_minimizer,
            port_distributor,
            crossings_counter,
            n_ports: 0,
            consider_model_order_strategy,
            group_order_strategy,
            port_model_order,
            node_influence,
            port_influence,
            thoroughness,
            first_try_with_initial_order: false,
            second_try_with_initial_order: false,
        };

        let order = holder.current_node_order.clone();
        {
            let mut holder_initializables: [&mut dyn IInitializable; 1] = [&mut holder];
            init_initializables(&mut holder_initializables, &order);
        }
        if trace {
            eprintln!("crossmin: graph_info_holder init holder done");
        }

        let mut initializables: Vec<&mut dyn IInitializable> = vec![
            &mut holder.crossings_counter,
            &mut layer_sweep_type_decider,
            &mut *holder.port_distributor,
            &mut *holder.cross_minimizer,
        ];

        init_initializables(&mut initializables, &order);
        if trace {
            eprintln!("crossmin: graph_info_holder init components done");
        }

        holder.use_bottom_up =
            layer_sweep_type_decider.use_bottom_up_with_boundary(&order, hierarchical_sweepiness);
        if trace {
            eprintln!("crossmin: graph_info_holder use_bottom_up={}", holder.use_bottom_up);
        }
        holder.update_greedy_context(None);

        holder
    }

    pub fn dont_sweep_into(&self) -> bool {
        self.use_bottom_up
    }

    pub fn l_graph(&self) -> &LGraphRef {
        &self.l_graph
    }

    pub fn current_node_order(&self) -> &Vec<Vec<LNodeRef>> {
        &self.current_node_order
    }

    pub fn current_node_order_mut(&mut self) -> &mut Vec<Vec<LNodeRef>> {
        &mut self.current_node_order
    }

    pub fn currently_best_node_and_port_order(&self) -> Option<&SweepCopy> {
        self.currently_best_node_and_port_order.as_ref()
    }

    pub fn set_currently_best_node_and_port_order(&mut self, sweep: SweepCopy) {
        self.currently_best_node_and_port_order = Some(sweep);
    }

    pub fn best_node_and_port_order(&self) -> Option<&SweepCopy> {
        self.best_node_and_port_order.as_ref()
    }

    pub fn set_best_node_and_port_order(&mut self, sweep: SweepCopy) {
        self.best_node_and_port_order = Some(sweep);
    }

    pub fn cross_counter(&mut self) -> &mut AllCrossingsCounter {
        &mut self.crossings_counter
    }

    pub fn cross_minimizer(&mut self) -> &mut dyn ICrossingMinimizationHeuristic {
        &mut *self.cross_minimizer
    }

    pub fn port_distributor(&mut self) -> &mut dyn ISweepPortDistributor {
        &mut *self.port_distributor
    }

    pub fn set_first_layer_order(&mut self, forward_sweep: bool) -> bool {
        self.cross_minimizer
            .set_first_layer_order(&mut self.current_node_order, forward_sweep)
    }

    pub fn minimize_crossings_on_layer(
        &mut self,
        free_layer_index: usize,
        forward_sweep: bool,
        is_first_sweep: bool,
    ) -> bool {
        self.cross_minimizer.minimize_crossings(
            &mut self.current_node_order,
            free_layer_index,
            forward_sweep,
            is_first_sweep,
        )
    }

    pub fn parent(&self) -> Option<LNodeRef> {
        self.parent_node.clone()
    }

    pub fn has_parent(&self) -> bool {
        self.has_parent
    }

    pub fn child_graphs(&self) -> &Vec<LGraphRef> {
        &self.child_graphs
    }

    pub fn has_external_ports(&self) -> bool {
        self.has_external_ports
    }

    pub fn best_sweep(&self) -> Option<&SweepCopy> {
        if self.cross_minimizer.is_deterministic() {
            self.currently_best_node_and_port_order.as_ref()
        } else {
            self.best_node_and_port_order.as_ref()
        }
    }

    pub fn parent_graph_index(&self) -> Option<usize> {
        self.parent_graph_index
    }

    pub fn parent_graph_ref(&self) -> Option<&LGraphRef> {
        self.parent_graph_ref.as_ref()
    }

    pub fn cross_min_deterministic(&self) -> bool {
        self.cross_minimizer.is_deterministic()
    }

    pub fn cross_min_always_improves(&self) -> bool {
        self.cross_minimizer.always_improves()
    }

    pub fn consider_model_order_strategy(&self) -> OrderingStrategy {
        self.consider_model_order_strategy
    }

    pub fn group_order_strategy(&self) -> GroupOrderStrategy {
        self.group_order_strategy
    }

    pub fn port_model_order(&self) -> bool {
        self.port_model_order
    }

    pub fn node_influence(&self) -> f64 {
        self.node_influence
    }

    pub fn port_influence(&self) -> f64 {
        self.port_influence
    }

    pub fn thoroughness(&self) -> i32 {
        self.thoroughness
    }

    pub fn first_try_with_initial_order(&self) -> bool {
        self.first_try_with_initial_order
    }

    pub fn second_try_with_initial_order(&self) -> bool {
        self.second_try_with_initial_order
    }

    pub fn set_first_try_with_initial_order(&mut self, value: bool) {
        self.first_try_with_initial_order = value;
    }

    pub fn set_second_try_with_initial_order(&mut self, value: bool) {
        self.second_try_with_initial_order = value;
    }

    pub fn set_parent_graph_index(&mut self, value: Option<usize>) {
        self.parent_graph_index = value;
    }

    pub fn port_positions(&self) -> &Vec<i32> {
        &self.port_positions
    }

    pub fn update_greedy_context(
        &mut self,
        parent_snapshot: Option<(Vec<Vec<LNodeRef>>, Vec<i32>, LNodeRef)>,
    ) {
        let Some(greedy) = self
            .cross_minimizer
            .as_any_mut()
            .downcast_mut::<GreedySwitchHeuristic>()
        else {
            return;
        };
        greedy.set_dont_sweep_into(self.use_bottom_up);
        if let Some((parent_node_order, parent_port_positions, parent_node)) = parent_snapshot {
            greedy.update_parent_context(parent_node_order, parent_port_positions, parent_node);
        }
    }
}

impl IInitializable for GraphInfoHolder {
    fn init_at_node_level(&mut self, layer_index: usize, node_index: usize, node_order: &[Vec<LNodeRef>]) {
        let node = node_order
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
            .cloned();
        if let Some(node) = node {
            if let Ok(node_guard) = node.lock() {
                if let Some(nested_graph) = node_guard.nested_graph() {
                    self.child_graphs.push(nested_graph);
                }
            }
        }
    }

    fn init_at_port_level(
        &mut self,
        _layer_index: usize,
        _node_index: usize,
        _port_index: usize,
        _node_order: &[Vec<LNodeRef>],
    ) {
        self.n_ports += 1;
    }

    fn init_after_traversal(&mut self) {
        self.port_positions = vec![0; self.n_ports];
    }
}

fn trace_in_layer_constraints(node_order: &[Vec<LNodeRef>]) {
    if std::env::var_os("ELK_TRACE_CROSSMIN_CONSTRAINTS").is_none() {
        return;
    }

    for (layer_index, layer) in node_order.iter().enumerate() {
        let layer_nodes = layer
            .iter()
            .map(node_debug_name)
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("rust-crossmin: layer[{layer_index}] nodes=[{layer_nodes}]");

        for node in layer {
            let (node_name, successors) = node
                .lock()
                .ok()
                .map(|mut node_guard| {
                    let name = node_guard.to_string();
                    let successors: Vec<LNodeRef> = node_guard
                        .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                        .unwrap_or_default();
                    (name, successors)
                })
                .unwrap_or_else(|| ("<poisoned-node>".to_owned(), Vec::new()));

            if successors.is_empty() {
                continue;
            }

            let successor_names = successors
                .iter()
                .map(node_debug_name)
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!("rust-crossmin:   constraint {node_name} -> [{successor_names}]");
        }
    }
}

fn node_debug_name(node: &LNodeRef) -> String {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.to_string())
        .unwrap_or_else(|| "<poisoned-node>".to_owned())
}

#[derive(Clone)]
struct SharedNodeRelativePortDistributor {
    inner: Arc<Mutex<NodeRelativePortDistributor>>,
}

impl SharedNodeRelativePortDistributor {
    fn from_inner(inner: Arc<Mutex<NodeRelativePortDistributor>>) -> Self {
        Self { inner }
    }
}

impl ISweepPortDistributor for SharedNodeRelativePortDistributor {
    fn distribute_ports_while_sweeping(
        &mut self,
        order: &[Vec<LNodeRef>],
        free_layer_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        self.inner
            .lock()
            .ok()
            .map(|mut distributor| {
                distributor.distribute_ports_while_sweeping(order, free_layer_index, is_forward_sweep)
            })
            .unwrap_or(false)
    }
}

impl BarycenterPortDistributor for SharedNodeRelativePortDistributor {
    fn calculate_port_ranks(
        &mut self,
        layer: &[LNodeRef],
        port_type: crate::org::eclipse::elk::alg::layered::options::PortType,
    ) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.calculate_port_ranks(layer, port_type);
        }
    }

    fn port_ranks(&self) -> Vec<f64> {
        self.inner
            .lock()
            .ok()
            .map(|distributor| distributor.port_ranks().clone())
            .unwrap_or_default()
    }
}

impl IInitializable for SharedNodeRelativePortDistributor {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_layer_level(layer_index, node_order);
        }
    }

    fn init_at_node_level(&mut self, layer_index: usize, node_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_node_level(layer_index, node_index, node_order);
        }
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_port_level(layer_index, node_index, port_index, node_order);
        }
    }

    fn init_after_traversal(&mut self) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_after_traversal();
        }
    }
}

#[derive(Clone)]
struct SharedLayerTotalPortDistributor {
    inner: Arc<Mutex<LayerTotalPortDistributor>>,
}

impl SharedLayerTotalPortDistributor {
    fn from_inner(inner: Arc<Mutex<LayerTotalPortDistributor>>) -> Self {
        Self { inner }
    }
}

impl ISweepPortDistributor for SharedLayerTotalPortDistributor {
    fn distribute_ports_while_sweeping(
        &mut self,
        order: &[Vec<LNodeRef>],
        free_layer_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        self.inner
            .lock()
            .ok()
            .map(|mut distributor| {
                distributor.distribute_ports_while_sweeping(order, free_layer_index, is_forward_sweep)
            })
            .unwrap_or(false)
    }
}

impl BarycenterPortDistributor for SharedLayerTotalPortDistributor {
    fn calculate_port_ranks(
        &mut self,
        layer: &[LNodeRef],
        port_type: crate::org::eclipse::elk::alg::layered::options::PortType,
    ) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.calculate_port_ranks(layer, port_type);
        }
    }

    fn port_ranks(&self) -> Vec<f64> {
        self.inner
            .lock()
            .ok()
            .map(|distributor| distributor.port_ranks().clone())
            .unwrap_or_default()
    }
}

impl IInitializable for SharedLayerTotalPortDistributor {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_layer_level(layer_index, node_order);
        }
    }

    fn init_at_node_level(&mut self, layer_index: usize, node_index: usize, node_order: &[Vec<LNodeRef>]) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_node_level(layer_index, node_index, node_order);
        }
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_at_port_level(layer_index, node_index, port_index, node_order);
        }
    }

    fn init_after_traversal(&mut self) {
        if let Ok(mut distributor) = self.inner.lock() {
            distributor.init_after_traversal();
        }
    }
}

fn create_port_distributors(
    cross_min_type: CrossMinType,
    random: &mut Random,
    num_layers: usize,
) -> (
    Box<dyn ISweepPortDistributor>,
    Option<Box<dyn BarycenterPortDistributor>>,
) {
    let trace = std::env::var_os("ELK_TRACE_CROSSMIN").is_some();
    if cross_min_type == CrossMinType::TwoSidedGreedySwitch {
        if trace {
            eprintln!("crossmin: port_distributor=GreedySwitch");
        }
        return (Box::new(GreedyPortDistributor::new()), None);
    }

    let use_node_relative = random.next_boolean();
    let needs_barycenter = cross_min_type == CrossMinType::Barycenter;
    if use_node_relative {
        if trace {
            eprintln!("crossmin: port_distributor=NodeRelative (barycenter={})", needs_barycenter);
        }
        let inner = Arc::new(Mutex::new(NodeRelativePortDistributor::new(num_layers)));
        let sweep = Box::new(SharedNodeRelativePortDistributor::from_inner(inner.clone()))
            as Box<dyn ISweepPortDistributor>;
        let barycenter = if needs_barycenter {
            Some(Box::new(SharedNodeRelativePortDistributor::from_inner(inner))
                as Box<dyn BarycenterPortDistributor>)
        } else {
            None
        };
        (sweep, barycenter)
    } else {
        if trace {
            eprintln!("crossmin: port_distributor=LayerTotal (barycenter={})", needs_barycenter);
        }
        let inner = Arc::new(Mutex::new(LayerTotalPortDistributor::new(num_layers)));
        let sweep = Box::new(SharedLayerTotalPortDistributor::from_inner(inner.clone()))
            as Box<dyn ISweepPortDistributor>;
        let barycenter = if needs_barycenter {
            Some(Box::new(SharedLayerTotalPortDistributor::from_inner(inner))
                as Box<dyn BarycenterPortDistributor>)
        } else {
            None
        };
        (sweep, barycenter)
    }
}
