use std::any::Any;
use std::cmp::Ordering;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::intermediate::CMGroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayerConstraint, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_heuristic::BarycenterHeuristic;
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_port_distributor::BarycenterPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;

pub struct ModelOrderBarycenterHeuristic {
    base: BarycenterHeuristic,
    bigger_than: BTreeMap<usize, BTreeSet<usize>>,
    smaller_than: BTreeMap<usize, BTreeSet<usize>>,
    force_model_order: bool,
    max_model_order_nodes: i32,
    group_order_strategy: GroupOrderStrategy,
}

impl ModelOrderBarycenterHeuristic {
    pub fn new(
        constraint_resolver: ForsterConstraintResolver,
        random: Random,
        port_distributor: Box<dyn BarycenterPortDistributor>,
        force_model_order: bool,
        max_model_order_nodes: i32,
        group_order_strategy: GroupOrderStrategy,
    ) -> Self {
        ModelOrderBarycenterHeuristic {
            base: BarycenterHeuristic::new(constraint_resolver, random, port_distributor),
            bigger_than: BTreeMap::new(),
            smaller_than: BTreeMap::new(),
            force_model_order,
            max_model_order_nodes,
            group_order_strategy,
        }
    }

    pub fn set_random(&mut self, random: Random) {
        self.base.set_random(random);
    }

    pub fn random(&self) -> Random {
        self.base.random()
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.base.set_random_seed(seed);
    }

    fn compare_based_on_transitive_dependencies(&mut self, n1: &LNodeRef, n2: &LNodeRef) -> i32 {
        let id1 = node_ptr_id(n1);
        let id2 = node_ptr_id(n2);

        // Initialize empty sets if they don't exist (matching Java behavior)
        match self.bigger_than.entry(id1) {
            Entry::Vacant(entry) => {
                entry.insert(BTreeSet::new());
            }
            Entry::Occupied(entry) => {
                if entry.get().contains(&id2) {
                    return 1;
                }
            }
        }
        match self.bigger_than.entry(id2) {
            Entry::Vacant(entry) => {
                entry.insert(BTreeSet::new());
            }
            Entry::Occupied(entry) => {
                if entry.get().contains(&id1) {
                    return -1;
                }
            }
        }
        match self.smaller_than.entry(id1) {
            Entry::Vacant(entry) => {
                entry.insert(BTreeSet::new());
            }
            Entry::Occupied(entry) => {
                if entry.get().contains(&id2) {
                    return -1;
                }
            }
        }
        match self.smaller_than.entry(id2) {
            Entry::Vacant(entry) => {
                entry.insert(BTreeSet::new());
            }
            Entry::Occupied(entry) => {
                if entry.get().contains(&id1) {
                    return 1;
                }
            }
        }
        0
    }

    fn compare_based_on_barycenter(&mut self, n1: &LNodeRef, n2: &LNodeRef) -> i32 {
        let s1 = self
            .base
            .state_of(n1)
            .and_then(|state| state.lock().ok().map(|state| state.barycenter));
        let s2 = self
            .base
            .state_of(n2)
            .and_then(|state| state.lock().ok().map(|state| state.barycenter));

        match (s1, s2) {
            (Some(s1), Some(s2)) => {
                let value = match s1.partial_cmp(&s2).unwrap_or(Ordering::Equal) {
                    Ordering::Less => -1,
                    Ordering::Greater => 1,
                    Ordering::Equal => 0,
                };
                if value < 0 {
                    self.update_bigger_and_smaller_associations(n1, n2);
                } else if value > 0 {
                    self.update_bigger_and_smaller_associations(n2, n1);
                }
                value
            }
            (Some(_), None) => {
                self.update_bigger_and_smaller_associations(n1, n2);
                -1
            }
            (None, Some(_)) => {
                self.update_bigger_and_smaller_associations(n2, n1);
                1
            }
            _ => 0,
        }
    }

    fn update_bigger_and_smaller_associations(&mut self, bigger: &LNodeRef, smaller: &LNodeRef) {
        let big_id = node_ptr_id(bigger);
        let small_id = node_ptr_id(smaller);

        // Capture the current state of all four sets BEFORE any modifications
        // In Java these are references, but we clone to avoid borrow checker issues
        // Java: biggerNodeBiggerThan, smallerNodeBiggerThan, biggerNodeSmallerThan, smallerNodeSmallerThan
        let smaller_node_bigger_than = self.bigger_than.get(&small_id).cloned().unwrap_or_default();
        let bigger_node_smaller_than = self.smaller_than.get(&big_id).cloned().unwrap_or_default();

        // biggerNodeBiggerThan.add(smaller)
        self.bigger_than.entry(big_id).or_default().insert(small_id);
        // smallerNodeSmallerThan.add(bigger)
        self.smaller_than
            .entry(small_id)
            .or_default()
            .insert(big_id);

        // for (LNode verySmall : smallerNodeBiggerThan)
        for very_small in smaller_node_bigger_than.iter() {
            // biggerNodeBiggerThan.add(verySmall)
            self.bigger_than
                .entry(big_id)
                .or_default()
                .insert(*very_small);
            // smallerThan.get(verySmall).add(bigger)
            self.smaller_than
                .entry(*very_small)
                .or_default()
                .insert(big_id);
            // smallerThan.get(verySmall).addAll(biggerNodeSmallerThan)
            self.smaller_than
                .entry(*very_small)
                .or_default()
                .extend(bigger_node_smaller_than.iter().copied());
        }

        // for (LNode veryBig : biggerNodeSmallerThan)
        for very_big in bigger_node_smaller_than.iter() {
            // smallerNodeSmallerThan.add(veryBig)
            self.smaller_than
                .entry(small_id)
                .or_default()
                .insert(*very_big);
            // biggerThan.get(veryBig).add(smaller)
            self.bigger_than
                .entry(*very_big)
                .or_default()
                .insert(small_id);
            // biggerThan.get(veryBig).addAll(smallerNodeBiggerThan)
            self.bigger_than
                .entry(*very_big)
                .or_default()
                .extend(smaller_node_bigger_than.iter().copied());
        }
    }

    fn compare_nodes(&mut self, n1: &LNodeRef, n2: &LNodeRef) -> i32 {
        let constraint1 = n1
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            })
            .unwrap_or(LayerConstraint::None);
        let constraint2 = n2
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            })
            .unwrap_or(LayerConstraint::None);
        if matches!(
            constraint1,
            LayerConstraint::FirstSeparate | LayerConstraint::LastSeparate
        ) || matches!(
            constraint2,
            LayerConstraint::FirstSeparate | LayerConstraint::LastSeparate
        ) {
            return 0;
        }

        let transitive = self.compare_based_on_transitive_dependencies(n1, n2);
        if transitive != 0 {
            return transitive;
        }

        let n1_has_model_order = n1
            .lock()
            .ok()
            .map(|mut node_guard| {
                node_guard
                    .get_property(InternalProperties::MODEL_ORDER)
                    .is_some()
            })
            .unwrap_or(false);
        let n2_has_model_order = n2
            .lock()
            .ok()
            .map(|mut node_guard| {
                node_guard
                    .get_property(InternalProperties::MODEL_ORDER)
                    .is_some()
            })
            .unwrap_or(false);

        if n1_has_model_order && n2_has_model_order {
            let graph = n1.lock().ok().and_then(|node_guard| node_guard.graph());
            if let Some(graph) = graph {
                let max_nodes = self.max_model_order_nodes;
                let value =
                    CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                        n1, n2, &graph, max_nodes,
                    )
                    .cmp(
                        &CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                            n2, n1, &graph, max_nodes,
                        ),
                    );
                let mut value = match value {
                    Ordering::Less => -1,
                    Ordering::Greater => 1,
                    Ordering::Equal => 0,
                };

                if self.group_order_strategy == GroupOrderStrategy::OnlyWithinGroup {
                    let n1_group = n1
                        .lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(
                                LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID,
                            )
                        })
                        .unwrap_or(0);
                    let n2_group = n2
                        .lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(
                                LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID,
                            )
                        })
                        .unwrap_or(0);
                    if n1_group != n2_group {
                        value = 0;
                    }
                }

                if value < 0 {
                    self.update_bigger_and_smaller_associations(n1, n2);
                    return value;
                } else if value > 0 {
                    self.update_bigger_and_smaller_associations(n2, n1);
                    return value;
                }
            }
        }

        self.compare_based_on_barycenter(n1, n2)
    }

    fn insertion_sort(&mut self, layer: &mut [LNodeRef]) {
        for i in 1..layer.len() {
            let temp = layer[i].clone();
            let mut j = i;
            while j > 0 && self.compare_nodes(&layer[j - 1], &temp) > 0 {
                layer[j] = layer[j - 1].clone();
                j -= 1;
            }
            layer[j] = temp;
        }
        self.clear_transitive_ordering();
    }

    pub fn clear_transitive_ordering(&mut self) {
        self.bigger_than.clear();
        self.smaller_than.clear();
    }
}

impl ICrossingMinimizationHeuristic for ModelOrderBarycenterHeuristic {
    fn always_improves(&self) -> bool {
        self.base.always_improves()
    }

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool) -> bool {
        self.base.set_first_layer_order(order, forward_sweep)
    }

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        is_first_sweep: bool,
    ) -> bool {
        if !self
            .base
            .is_first_layer(order, free_layer_index, forward_sweep)
        {
            let fixed_layer_index = if forward_sweep {
                free_layer_index.saturating_sub(1)
            } else {
                free_layer_index + 1
            };
            let port_type = if forward_sweep {
                crate::org::eclipse::elk::alg::layered::options::PortType::Output
            } else {
                crate::org::eclipse::elk::alg::layered::options::PortType::Input
            };
            if let Some(layer) = order.get(fixed_layer_index) {
                self.base
                    .port_distributor
                    .calculate_port_ranks(layer, port_type);
                self.base.port_ranks = self.base.port_distributor.port_ranks();
            }
        }

        let pre_ordered = !is_first_sweep
            || order
                .get(free_layer_index)
                .and_then(|layer| layer.first())
                .map(|node| self.base.is_external_port_dummy(node))
                .unwrap_or(false);

        let mut nodes = order[free_layer_index].clone();
        self.base.calculate_barycenters(&nodes, forward_sweep);
        self.base.fill_in_unknown_barycenters(&nodes, pre_ordered);

        if nodes.len() > 1 {
            if self.force_model_order {
                self.insertion_sort(&mut nodes);
            } else {
                nodes.sort_by(|a, b| {
                    let value = self.compare_nodes(a, b);
                    if value < 0 {
                        Ordering::Less
                    } else if value > 0 {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                });
                self.base
                    .constraint_resolver
                    .process_constraints(&mut nodes);
            }
        }

        order[free_layer_index] = nodes;
        false
    }

    fn is_deterministic(&self) -> bool {
        self.base.is_deterministic()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl IInitializable for ModelOrderBarycenterHeuristic {
    fn init_after_traversal(&mut self) {
        self.base.init_after_traversal();
    }

    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        self.base.init_at_layer_level(layer_index, node_order);
    }

    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.base
            .init_at_node_level(layer_index, node_index, node_order);
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.base
            .init_at_port_level(layer_index, node_index, port_index, node_order);
    }

    fn init_at_edge_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        edge_index: usize,
        edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.base.init_at_edge_level(
            layer_index,
            node_index,
            port_index,
            edge_index,
            edge,
            node_order,
        );
    }
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    std::sync::Arc::as_ptr(node) as usize
}
