#![allow(clippy::mutable_key_type)]

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::org::eclipse::elk::alg::layered::graph::{LGraphRef, LNodeRef, LPortRef, NodeRefKey, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::preserveorder::model_order_port_comparator::ModelOrderPortComparator;
use crate::org::eclipse::elk::alg::layered::intermediate::CMGroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LongEdgeOrderingStrategy, OrderingStrategy,
};

static TRACE_CROSSMIN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());

pub struct ModelOrderNodeComparator {
    previous_layer: Vec<LNodeRef>,
    previous_layer_position: HashMap<usize, usize>,
    graph: LGraphRef,
    ordering_strategy: OrderingStrategy,
    _group_order_strategy: GroupOrderStrategy,
    bigger_than: HashMap<NodeRefKey, HashSet<NodeRefKey>>,
    smaller_than: HashMap<NodeRefKey, HashSet<NodeRefKey>>,
    long_edge_node_order: LongEdgeOrderingStrategy,
    before_ports: bool,
    visiting: HashSet<NodePairKey>,
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
        let mut previous_layer_position = HashMap::with_capacity(previous_layer.len());
        refill_layer_position_map(&mut previous_layer_position, &previous_layer);
        ModelOrderNodeComparator {
            previous_layer,
            previous_layer_position,
            graph,
            ordering_strategy,
            _group_order_strategy: group_order_strategy,
            bigger_than: HashMap::new(),
            smaller_than: HashMap::new(),
            long_edge_node_order,
            before_ports,
            visiting: HashSet::new(),
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
        self.ensure_sets(&n1_key);
        self.ensure_sets(&n2_key);
        if self
            .bigger_than
            .get(&n1_key)
            .is_some_and(|set| set.contains(&n2_key))
        {
            return self.finish_compare(&pair_key, 1);
        }
        if self
            .bigger_than
            .get(&n2_key)
            .is_some_and(|set| set.contains(&n1_key))
        {
            return self.finish_compare(&pair_key, -1);
        }
        if self
            .smaller_than
            .get(&n1_key)
            .is_some_and(|set| set.contains(&n2_key))
        {
            return self.finish_compare(&pair_key, -1);
        }
        if self
            .smaller_than
            .get(&n2_key)
            .is_some_and(|set| set.contains(&n1_key))
        {
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
                let p1_node = p1_source_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                let p2_node = p2_source_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                if let (Some(p1_node), Some(p2_node)) = (&p1_node, &p2_node) {
                    if ArcPtr::eq_nodes(p1_node, p2_node) {
                        let ports = p1_node
                            .lock()
                            .ok()
                            .map(|node_guard| node_guard.ports().clone())
                            .unwrap_or_default();
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
                Ok(mut graph_guard) => graph_guard
                    .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                    .unwrap_or(0),
                Err(_) => {
                    if *TRACE_CROSSMIN {
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
        self.bigger_than.clear();
        self.smaller_than.clear();
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

    fn ensure_sets(&mut self, key: &NodeRefKey) {
        self.bigger_than.entry(key.clone()).or_default();
        self.smaller_than.entry(key.clone()).or_default();
    }

    fn finish_compare(&mut self, key: &NodePairKey, value: i32) -> i32 {
        self.visiting.remove(key);
        value
    }

    fn compare_in_previous_layer(&self, p1_node: &LNodeRef, p2_node: &LNodeRef) -> i32 {
        let p1_pos = self.previous_layer_position.get(&node_ptr(p1_node)).copied();
        let p2_pos = self.previous_layer_position.get(&node_ptr(p2_node)).copied();
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

    fn model_order_from_connected_edges(&self, node: &LNodeRef) -> i32 {
        let source_port = first_incoming_port(node);
        if let Some(port) = source_port {
            let edge = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.incoming_edges().first().cloned());
            if let Some(edge) = edge {
                let order = edge.lock().ok().and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::MODEL_ORDER)
                });
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
            let Some(dummy_source_node) = dummy_source_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
                return 0;
            };
            let Some(dummy_target_node) = dummy_target_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
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
            let Some(dummy_source_node) = dummy_source_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
                return 0;
            };
            let Some(dummy_target_node) = dummy_target_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
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
            let Some(n1_source_node) = n1_source_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
                return 0;
            };
            let Some(n1_target_node) = n1_target_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
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
            let Some(n2_source_node) = n2_source_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
                return 0;
            };
            let Some(n2_target_node) = n2_target_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.node())
            else {
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
                    let ports = n1_reference
                        .lock()
                        .ok()
                        .map(|node_guard| node_guard.ports().clone())
                        .unwrap_or_default();
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
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::MODEL_ORDER))
        .is_some()
}

fn first_incoming_port(node: &LNodeRef) -> Option<LPortRef> {
    node.lock().ok().and_then(|node_guard| {
        node_guard
            .ports()
            .iter()
            .find(|port| {
                port.lock()
                    .ok()
                    .map(|port_guard| !port_guard.incoming_edges().is_empty())
                    .unwrap_or(false)
            })
            .cloned()
    })
}

fn first_incoming_source_port(node: &LNodeRef) -> Option<LPortRef> {
    let port = first_incoming_port(node)?;
    port.lock()
        .ok()
        .and_then(|port_guard| port_guard.incoming_edges().first().cloned())
        .and_then(|edge| edge.lock().ok().and_then(|edge_guard| edge_guard.source()))
}

fn first_outgoing_port(node: &LNodeRef) -> Option<LPortRef> {
    node.lock().ok().and_then(|node_guard| {
        node_guard
            .ports()
            .iter()
            .find(|port| {
                port.lock()
                    .ok()
                    .map(|port_guard| !port_guard.outgoing_edges().is_empty())
                    .unwrap_or(false)
            })
            .cloned()
    })
}

fn first_outgoing_target_port(node: &LNodeRef) -> Option<LPortRef> {
    let port = first_outgoing_port(node)?;
    port.lock()
        .ok()
        .and_then(|port_guard| port_guard.outgoing_edges().first().cloned())
        .and_then(|edge| edge.lock().ok().and_then(|edge_guard| edge_guard.target()))
}

fn first_source_port_to_previous_layer(node: &LNodeRef) -> Option<LPortRef> {
    let node_layer = layer_id(node);
    let prev_layer = node_layer.checked_sub(1)?;
    let ports = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    for port in ports {
        let incoming = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.incoming_edges().clone())
            .unwrap_or_default();
        if let Some(edge) = incoming.first() {
            let source_port = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
            if let Some(source_port) = source_port {
                let source_node = source_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
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
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
        .unwrap_or(0)
}

fn node_type(node: &LNodeRef) -> NodeType {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(NodeType::Normal)
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

fn refill_layer_position_map(positions: &mut HashMap<usize, usize>, layer: &[LNodeRef]) {
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
