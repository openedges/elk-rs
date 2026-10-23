#![allow(clippy::mutable_key_type)]

use rustc_hash::{FxHashMap, FxHashSet};

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;
use crate::org::eclipse::elk::alg::layered::graph::{LGraphRef, LNodeRef, LPortRef, NodeRefKey, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::preserveorder::model_order_port_comparator::ModelOrderPortComparator;
use crate::org::eclipse::elk::alg::layered::intermediate::CMGroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LongEdgeOrderingStrategy, OrderingStrategy,
};

pub struct ModelOrderNodeComparator {
    previous_layer: Vec<LNodeRef>,
    previous_layer_position: FxHashMap<usize, usize>,
    graph: LGraphRef,
    ordering_strategy: OrderingStrategy,
    _group_order_strategy: GroupOrderStrategy,
    relations: ModelOrderRelations<NodeRefKey>,
    long_edge_node_order: LongEdgeOrderingStrategy,
    before_ports: bool,
    visiting: FxHashSet<NodePairKey>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct NodePairKey(NodeRefKey, NodeRefKey);

impl ModelOrderNodeComparator {
    pub fn new(
        graph: LGraphRef,
        previous_layer: Vec<LNodeRef>,
        ordering_strategy: OrderingStrategy,
        long_edge_node_order: LongEdgeOrderingStrategy,
        group_order_strategy: GroupOrderStrategy,
        before_ports: bool,
    ) -> Self {
        let mut previous_layer_position =
            FxHashMap::with_capacity_and_hasher(previous_layer.len(), Default::default());
        refill_layer_position_map(&mut previous_layer_position, &previous_layer);
        ModelOrderNodeComparator {
            previous_layer,
            previous_layer_position,
            graph,
            ordering_strategy,
            _group_order_strategy: group_order_strategy,
            relations: ModelOrderRelations::default(),
            long_edge_node_order,
            before_ports,
            visiting: FxHashSet::default(),
        }
    }

    pub fn compare(&mut self, n1: &LNodeRef, n2: &LNodeRef) -> i32 {
        let n1_key = NodeRefKey(n1.clone());
        let n2_key = NodeRefKey(n2.clone());
        let pair_key = NodePairKey(n1_key.clone(), n2_key.clone());
        if self.visiting.contains(&pair_key) {
            return 0;
        }
        self.visiting.insert(pair_key.clone());
        let n1_index = self.relations.index(&n1_key);
        let n2_index = self.relations.index(&n2_key);
        if self.relations.bigger(n1_index, n2_index) {
            return self.finish_compare(&pair_key, 1);
        }
        if self.relations.bigger(n2_index, n1_index) {
            return self.finish_compare(&pair_key, -1);
        }
        if self.relations.smaller(n1_index, n2_index) {
            return self.finish_compare(&pair_key, -1);
        }
        if self.relations.smaller(n2_index, n1_index) {
            return self.finish_compare(&pair_key, 1);
        }

        let n1_has_model_order = has_model_order(n1);
        let n2_has_model_order = has_model_order(n2);
        if self.ordering_strategy == OrderingStrategy::PreferEdges
            || !n1_has_model_order
            || !n2_has_model_order
        {
            let p1_source_port = first_source_port_to_previous_layer(n1);
            let p2_source_port = first_source_port_to_previous_layer(n2);

            if let (Some(p1_source_port), Some(p2_source_port)) = (&p1_source_port, &p2_source_port)
            {
                let p1_node = p1_source_port.lock().node();
                let p2_node = p2_source_port.lock().node();
                if let (Some(p1_node), Some(p2_node)) = (&p1_node, &p2_node) {
                    if ArcPtr::eq_nodes(p1_node, p2_node) {
                        let ports = p1_node.lock().ports().clone();
                        for port in ports {
                            if ArcPtr::eq_ports(&port, p1_source_port) {
                                self.update_bigger_and_smaller(n2, n1);
                                return self.finish_compare(&pair_key, -1);
                            } else if ArcPtr::eq_ports(&port, p2_source_port) {
                                self.update_bigger_and_smaller(n1, n2);
                                return self.finish_compare(&pair_key, 1);
                            }
                        }
                        let n1_edge_order = self.model_order_from_connected_edges(n1);
                        let n2_edge_order = self.model_order_from_connected_edges(n2);
                        if n1_edge_order > n2_edge_order {
                            self.update_bigger_and_smaller(n1, n2);
                            return self.finish_compare(&pair_key, 1);
                        } else {
                            self.update_bigger_and_smaller(n2, n1);
                            return self.finish_compare(&pair_key, -1);
                        }
                    }

                    let in_previous = self.compare_in_previous_layer(p1_node, p2_node);
                    if in_previous != 0 {
                        if in_previous > 0 {
                            self.update_bigger_and_smaller(n1, n2);
                            return self.finish_compare(&pair_key, 1);
                        } else {
                            self.update_bigger_and_smaller(n2, n1);
                            return self.finish_compare(&pair_key, -1);
                        }
                    }
                }
            }

            if p1_source_port.is_some() ^ p2_source_port.is_some() {
                let compared = self.handle_helper_dummy_nodes(n1, n2);
                if compared != 0 {
                    if compared > 0 {
                        self.update_bigger_and_smaller(n1, n2);
                    } else {
                        self.update_bigger_and_smaller(n2, n1);
                    }
                    return self.finish_compare(&pair_key, compared);
                }

                if !n1_has_model_order || !n2_has_model_order {
                    let n1_edge_order = self.model_order_from_connected_edges(n1);
                    let n2_edge_order = self.model_order_from_connected_edges(n2);
                    if n1_edge_order > n2_edge_order {
                        self.update_bigger_and_smaller(n1, n2);
                        return self.finish_compare(&pair_key, 1);
                    } else {
                        self.update_bigger_and_smaller(n2, n1);
                        return self.finish_compare(&pair_key, -1);
                    }
                }
            }

            if p1_source_port.is_none() && p2_source_port.is_none() {
                let compared = self.handle_helper_dummy_nodes(n1, n2);
                if compared != 0 {
                    if compared > 0 {
                        self.update_bigger_and_smaller(n1, n2);
                    } else {
                        self.update_bigger_and_smaller(n2, n1);
                    }
                    return self.finish_compare(&pair_key, compared);
                }
            }
        }

        if n1_has_model_order && n2_has_model_order {
            let max_nodes = match self.graph.try_lock() {
                Some(graph_guard) => graph_guard
                    .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                    .unwrap_or(0),
                None => {
                    if ElkTrace::global().crossmin {
                        eprintln!("node_compare: graph lock busy, using default max_nodes");
                    }
                    0
                }
            };
            let n1_order = CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                n1,
                n2,
                &self.graph,
                max_nodes,
            );
            let n2_order = CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                n2,
                n1,
                &self.graph,
                max_nodes,
            );
            if n1_order > n2_order {
                self.update_bigger_and_smaller(n1, n2);
                return self.finish_compare(&pair_key, 1);
            }
            self.update_bigger_and_smaller(n2, n1);
            return self.finish_compare(&pair_key, -1);
        }

        self.update_bigger_and_smaller(n2, n1);
        self.finish_compare(&pair_key, -1)
    }

    pub fn clear_transitive_ordering(&mut self) {
        self.relations = ModelOrderRelations::default();
        self.visiting.clear();
    }

    pub fn reset_for_previous_layer(&mut self, previous_layer: Vec<LNodeRef>) {
        self.previous_layer = previous_layer;
        refill_layer_position_map(&mut self.previous_layer_position, &self.previous_layer);
        self.clear_transitive_ordering();
    }

    pub fn reset_for_previous_layer_slice(&mut self, previous_layer: &[LNodeRef]) {
        self.previous_layer.clear();
        self.previous_layer.extend(previous_layer.iter().cloned());
        refill_layer_position_map(&mut self.previous_layer_position, &self.previous_layer);
        self.clear_transitive_ordering();
    }

    fn finish_compare(&mut self, key: &NodePairKey, value: i32) -> i32 {
        self.visiting.remove(key);
        value
    }

    fn compare_in_previous_layer(&self, p1_node: &LNodeRef, p2_node: &LNodeRef) -> i32 {
        let p1_pos = self
            .previous_layer_position
            .get(&node_ptr(p1_node))
            .copied();
        let p2_pos = self
            .previous_layer_position
            .get(&node_ptr(p2_node))
            .copied();
        match (p1_pos, p2_pos) {
            (Some(left), Some(right)) => match left.cmp(&right) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            },
            _ => 0,
        }
    }

    fn update_bigger_and_smaller(&mut self, bigger: &LNodeRef, smaller: &LNodeRef) {
        let bigger = self.relations.index(&NodeRefKey(bigger.clone()));
        let smaller = self.relations.index(&NodeRefKey(smaller.clone()));
        self.relations.record(bigger, smaller);
    }

    fn model_order_from_connected_edges(&self, node: &LNodeRef) -> i32 {
        let source_port = first_incoming_port(node);
        if let Some(port) = source_port {
            let edge = {
                let port_guard = port.lock();
                port_guard.incoming_edges().first().cloned()
            };
            if let Some(edge) = edge {
                let order = {
                    let edge_guard = edge.lock();
                    edge_guard.get_property(InternalProperties::MODEL_ORDER)
                };
                if let Some(order) = order {
                    return order;
                }
            }
        }
        self.long_edge_node_order.return_value()
    }

    fn handle_helper_dummy_nodes(&mut self, n1: &LNodeRef, n2: &LNodeRef) -> i32 {
        let n1_type = node_type(n1);
        let n2_type = node_type(n2);
        if n1_type == NodeType::LongEdge && n2_type == NodeType::Normal {
            let Some(dummy_source_port) = first_incoming_source_port(n1) else {
                return 0;
            };
            let Some(dummy_target_port) = first_outgoing_target_port(n1) else {
                return 0;
            };
            let Some(dummy_source_node) = dummy_source_port.lock().node() else {
                return 0;
            };
            let Some(dummy_target_node) = dummy_target_port.lock().node() else {
                return 0;
            };
            let dummy_layer_id = layer_id(n1);
            if layer_id(&dummy_source_node) != dummy_layer_id
                && layer_id(&dummy_target_node) != dummy_layer_id
            {
                return 0;
            }
            if ArcPtr::eq_nodes(&dummy_source_node, n2) {
                self.update_bigger_and_smaller(n1, n2);
                return 1;
            }
            if ArcPtr::eq_nodes(&dummy_target_node, n2) {
                self.update_bigger_and_smaller(n1, n2);
                return 1;
            }
            return self.compare(&dummy_source_node, n2);
        } else if n1_type == NodeType::Normal && n2_type == NodeType::LongEdge {
            let Some(dummy_source_port) = first_incoming_source_port(n2) else {
                return 0;
            };
            let Some(dummy_target_port) = first_outgoing_target_port(n2) else {
                return 0;
            };
            let Some(dummy_source_node) = dummy_source_port.lock().node() else {
                return 0;
            };
            let Some(dummy_target_node) = dummy_target_port.lock().node() else {
                return 0;
            };
            let dummy_layer_id = layer_id(n1);
            if layer_id(&dummy_source_node) != dummy_layer_id
                && layer_id(&dummy_target_node) != dummy_layer_id
            {
                return 0;
            }
            if ArcPtr::eq_nodes(&dummy_source_node, n1) {
                self.update_bigger_and_smaller(n2, n1);
                return -1;
            }
            if ArcPtr::eq_nodes(&dummy_target_node, n1) {
                self.update_bigger_and_smaller(n2, n1);
                return -1;
            }
            return self.compare(n1, &dummy_source_node);
        } else if n1_type == NodeType::LongEdge && n2_type == NodeType::LongEdge {
            let Some(n1_source_port) = first_incoming_source_port(n1) else {
                return 0;
            };
            let Some(n1_target_port) = first_outgoing_target_port(n1) else {
                return 0;
            };
            let Some(n1_source_node) = n1_source_port.lock().node() else {
                return 0;
            };
            let Some(n1_target_node) = n1_target_port.lock().node() else {
                return 0;
            };
            let n1_layer_id = layer_id(n1);
            let mut n1_source_feedback = false;
            let mut n1_target_feedback = false;
            let mut n1_reference = n1.clone();

            if layer_id(&n1_source_node) == n1_layer_id {
                n1_source_feedback = true;
                n1_reference = n1_source_node.clone();
            } else if layer_id(&n1_target_node) == n1_layer_id {
                n1_target_feedback = true;
                n1_reference = n1_target_node.clone();
            }

            let Some(n2_source_port) = first_incoming_source_port(n2) else {
                return 0;
            };
            let Some(n2_target_port) = first_outgoing_target_port(n2) else {
                return 0;
            };
            let Some(n2_source_node) = n2_source_port.lock().node() else {
                return 0;
            };
            let Some(n2_target_node) = n2_target_port.lock().node() else {
                return 0;
            };
            let n2_layer_id = layer_id(n2);
            let mut n2_source_feedback = false;
            let mut n2_target_feedback = false;
            let mut n2_reference = n2.clone();

            if layer_id(&n2_source_node) == n2_layer_id {
                n2_source_feedback = true;
                n2_reference = n2_source_node.clone();
            } else if layer_id(&n2_target_node) == n2_layer_id {
                n2_target_feedback = true;
                n2_reference = n2_target_node.clone();
            }

            if ArcPtr::eq_nodes(&n1_reference, &n2_reference) {
                if self.before_ports {
                    if n1_source_feedback && n2_source_feedback {
                        let mut comparator = ModelOrderPortComparator::new(
                            self.graph.clone(),
                            self.previous_layer.clone(),
                            self.ordering_strategy,
                            None,
                            n2_target_feedback,
                        );
                        let return_value = comparator.compare(&n1_source_port, &n2_source_port);
                        if return_value > 0 {
                            self.update_bigger_and_smaller(n2, n1);
                            return 1;
                        }
                        self.update_bigger_and_smaller(n1, n2);
                        return -1;
                    } else if n1_source_feedback && n2_target_feedback {
                        self.update_bigger_and_smaller(n2, n1);
                        return 1;
                    } else if n1_target_feedback && n2_source_feedback {
                        self.update_bigger_and_smaller(n1, n2);
                        return -1;
                    } else if n1_target_feedback && n2_target_feedback {
                        return 0;
                    }
                } else {
                    let ports = n1_reference.lock().ports().clone();
                    for port in ports {
                        if ArcPtr::eq_ports(&port, &n1_source_port) {
                            self.update_bigger_and_smaller(n2, n1);
                            return -1;
                        } else if ArcPtr::eq_ports(&port, &n2_source_port) {
                            self.update_bigger_and_smaller(n1, n2);
                            return 1;
                        }
                    }
                }
            }

            return self.compare(&n1_reference, &n2_reference);
        }
        0
    }
}

fn has_model_order(node: &LNodeRef) -> bool {
    let node_guard = node.lock();
    node_guard
        .get_property(InternalProperties::MODEL_ORDER)
        .is_some()
}

fn first_incoming_port(node: &LNodeRef) -> Option<LPortRef> {
    let node_guard = node.lock();
    node_guard
        .ports()
        .iter()
        .find(|port| {
            let port_guard = port.lock();
            !port_guard.incoming_edges().is_empty()
        })
        .cloned()
}

fn first_incoming_source_port(node: &LNodeRef) -> Option<LPortRef> {
    let port = first_incoming_port(node)?;
    let edge = {
        let port_guard = port.lock();
        port_guard.incoming_edges().first().cloned()
    };
    edge.and_then(|edge| edge.lock().source())
}

fn first_outgoing_port(node: &LNodeRef) -> Option<LPortRef> {
    let node_guard = node.lock();
    node_guard
        .ports()
        .iter()
        .find(|port| {
            let port_guard = port.lock();
            !port_guard.outgoing_edges().is_empty()
        })
        .cloned()
}

fn first_outgoing_target_port(node: &LNodeRef) -> Option<LPortRef> {
    let port = first_outgoing_port(node)?;
    let edge = {
        let port_guard = port.lock();
        port_guard.outgoing_edges().first().cloned()
    };
    edge.and_then(|edge| edge.lock().target())
}

fn first_source_port_to_previous_layer(node: &LNodeRef) -> Option<LPortRef> {
    let node_layer = layer_id(node);
    let prev_layer = node_layer.checked_sub(1)?;
    let ports = node.lock().ports().clone();
    for port in ports {
        let incoming = port.lock().incoming_edges().clone();
        if let Some(edge) = incoming.first() {
            let source_port = edge.lock().source();
            if let Some(source_port) = source_port {
                let source_node = source_port.lock().node();
                if let Some(source_node) = source_node {
                    if layer_id(&source_node) == prev_layer {
                        return Some(source_port);
                    }
                }
            }
        }
    }
    None
}

fn layer_id(node: &LNodeRef) -> usize {
    node.lock()
        .layer()
        .map(|layer| {
            let mut layer_guard = layer.lock();
            layer_guard.graph_element().id as usize
        })
        .unwrap_or(0)
}

fn node_type(node: &LNodeRef) -> NodeType {
    node.lock().node_type()
}

struct ArcPtr;

impl ArcPtr {
    fn eq_nodes(a: &LNodeRef, b: &LNodeRef) -> bool {
        std::sync::Arc::ptr_eq(a, b)
    }

    fn eq_ports(a: &LPortRef, b: &LPortRef) -> bool {
        std::sync::Arc::ptr_eq(a, b)
    }
}

fn refill_layer_position_map(positions: &mut FxHashMap<usize, usize>, layer: &[LNodeRef]) {
    positions.clear();
    if positions.capacity() < layer.len() {
        positions.reserve(layer.len() - positions.capacity());
    }
    for (index, node) in layer.iter().enumerate() {
        positions.insert(node_ptr(node), index);
    }
}

fn node_ptr(node: &LNodeRef) -> usize {
    std::sync::Arc::as_ptr(node) as usize
}

// The original transitive-set updates, stored as dense bits rather than sets
// of owning graph references. Unions visit machine words instead of cloning and
// hashing every transitive pair; comparator and insertion-sort rules are unchanged.
pub(super) struct ModelOrderRelations<Key> {
    indices: FxHashMap<Key, usize>,
    bigger: Vec<Vec<u64>>,
    smaller: Vec<Vec<u64>>,
}

impl<Key> Default for ModelOrderRelations<Key> {
    fn default() -> Self {
        Self {
            indices: FxHashMap::default(),
            bigger: Vec::new(),
            smaller: Vec::new(),
        }
    }
}

impl<Key: Eq + std::hash::Hash + Clone> ModelOrderRelations<Key> {
    pub(super) fn index(&mut self, key: &Key) -> usize {
        if let Some(&index) = self.indices.get(key) {
            return index;
        }
        let index = self.indices.len();
        self.bigger.push(Vec::new());
        self.smaller.push(Vec::new());
        self.indices.insert(key.clone(), index);
        index
    }

    pub(super) fn bigger(&self, left: usize, right: usize) -> bool {
        self.bigger[left]
            .get(right / 64)
            .is_some_and(|word| word & (1 << (right % 64)) != 0)
    }

    pub(super) fn smaller(&self, left: usize, right: usize) -> bool {
        self.smaller[left]
            .get(right / 64)
            .is_some_and(|word| word & (1 << (right % 64)) != 0)
    }

    pub(super) fn record(&mut self, bigger: usize, smaller: usize) {
        // Snapshot before either direct insertion, exactly as the set-based
        // implementation does, including inconsistent/cyclic observations.
        let below = self.bigger[smaller].clone();
        let above = self.smaller[bigger].clone();
        set_insert(&mut self.bigger[bigger], smaller);
        set_insert(&mut self.smaller[smaller], bigger);
        for very_small in set_members(&below) {
            set_insert(&mut self.bigger[bigger], very_small);
            set_insert(&mut self.smaller[very_small], bigger);
            if self.smaller[very_small].len() < above.len() {
                self.smaller[very_small].resize(above.len(), 0);
            }
            for (word, extra) in self.smaller[very_small].iter_mut().zip(&above) {
                *word |= extra;
            }
        }
        for very_big in set_members(&above) {
            set_insert(&mut self.smaller[smaller], very_big);
            set_insert(&mut self.bigger[very_big], smaller);
            if self.bigger[very_big].len() < below.len() {
                self.bigger[very_big].resize(below.len(), 0);
            }
            for (word, extra) in self.bigger[very_big].iter_mut().zip(&below) {
                *word |= extra;
            }
        }
    }
}

fn set_insert(words: &mut Vec<u64>, member: usize) {
    let index = member / 64;
    if words.len() <= index {
        words.resize(index + 1, 0);
    }
    words[index] |= 1 << (member % 64);
}

fn set_members(words: &[u64]) -> impl Iterator<Item = usize> + '_ {
    words.iter().enumerate().flat_map(|(index, &word)| {
        let mut remaining = word;
        std::iter::from_fn(move || {
            if remaining == 0 {
                return None;
            }
            let bit = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            Some(index * 64 + bit)
        })
    })
}

#[cfg(test)]
mod relation_tests {
    use super::*;
    use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode};

    #[derive(Default)]
    struct Reference {
        bigger_than: FxHashMap<NodeRefKey, FxHashSet<NodeRefKey>>,
        smaller_than: FxHashMap<NodeRefKey, FxHashSet<NodeRefKey>>,
    }
    impl Reference {
        fn ensure_sets(&mut self, key: &NodeRefKey) {
            self.bigger_than.entry(key.clone()).or_default();
            self.smaller_than.entry(key.clone()).or_default();
        }

        fn update_bigger_and_smaller(&mut self, bigger: &LNodeRef, smaller: &LNodeRef) {
            let bigger_key = NodeRefKey(bigger.clone());
            let smaller_key = NodeRefKey(smaller.clone());
            self.ensure_sets(&bigger_key);
            self.ensure_sets(&smaller_key);
            let smaller_set = self
                .bigger_than
                .get(&smaller_key)
                .cloned()
                .unwrap_or_default();
            let bigger_smaller = self
                .smaller_than
                .get(&bigger_key)
                .cloned()
                .unwrap_or_default();

            self.bigger_than
                .entry(bigger_key.clone())
                .or_default()
                .insert(smaller_key.clone());
            self.smaller_than
                .entry(smaller_key.clone())
                .or_default()
                .insert(bigger_key.clone());

            for very_small in smaller_set.iter() {
                self.bigger_than
                    .entry(bigger_key.clone())
                    .or_default()
                    .insert(very_small.clone());
                self.smaller_than
                    .entry(very_small.clone())
                    .or_default()
                    .insert(bigger_key.clone());
                self.smaller_than
                    .entry(very_small.clone())
                    .or_default()
                    .extend(bigger_smaller.iter().cloned());
            }

            for very_big in bigger_smaller.iter() {
                self.smaller_than
                    .entry(smaller_key.clone())
                    .or_default()
                    .insert(very_big.clone());
                self.bigger_than
                    .entry(very_big.clone())
                    .or_default()
                    .insert(smaller_key.clone());
                self.bigger_than
                    .entry(very_big.clone())
                    .or_default()
                    .extend(smaller_set.iter().cloned());
            }
        }
    }

    #[test]
    fn dense_relations_match_original_sets_through_word_growth_and_cycles() {
        let graph = LGraph::new();
        let nodes: Vec<_> = (0..130).map(|_| NodeRefKey(LNode::new(&graph))).collect();
        let mut dense = ModelOrderRelations::default();
        let mut original = Reference::default();
        let mut state = 17_u64;
        for step in 0..400 {
            let (bigger, smaller) = if step < 129 {
                (step + 1, step)
            } else {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (
                    (state >> 32) as usize % nodes.len(),
                    state as usize % nodes.len(),
                )
            };
            let a = dense.index(&nodes[bigger]);
            let b = dense.index(&nodes[smaller]);
            dense.record(a, b);
            original.update_bigger_and_smaller(&nodes[bigger].0, &nodes[smaller].0);
            for (left, &li) in &dense.indices {
                for (right, &ri) in &dense.indices {
                    assert_eq!(
                        dense.bigger(li, ri),
                        original
                            .bigger_than
                            .get(left)
                            .is_some_and(|set| set.contains(right)),
                        "bigger at step={step}"
                    );
                    assert_eq!(
                        dense.smaller(li, ri),
                        original
                            .smaller_than
                            .get(left)
                            .is_some_and(|set| set.contains(right)),
                        "smaller at step={step}"
                    );
                }
            }
        }
    }
}
