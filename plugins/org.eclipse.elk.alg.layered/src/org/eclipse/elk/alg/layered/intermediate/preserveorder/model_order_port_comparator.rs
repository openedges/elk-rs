#![allow(clippy::mutable_key_type)]

use rustc_hash::{FxHashMap, FxHashSet};

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraphRef, LNodeRef, LPortRef, NodeRefKey,
};
use crate::org::eclipse::elk::alg::layered::intermediate::CMGroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, OrderingStrategy,
};

#[derive(Clone)]
struct PortRefKey(LPortRef);

impl PartialEq for PortRefKey {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PortRefKey {}

impl std::hash::Hash for PortRefKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = std::sync::Arc::as_ptr(&self.0) as usize;
        ptr.hash(state);
    }
}

pub struct ModelOrderPortComparator {
    target_node_model_order: Option<FxHashMap<usize, i32>>,
    port_model_order: bool,
    previous_layer: Vec<LNodeRef>,
    previous_layer_position: FxHashMap<usize, usize>,
    graph: LGraphRef,
    strategy: OrderingStrategy,
    bigger_than: FxHashMap<PortRefKey, FxHashSet<PortRefKey>>,
    smaller_than: FxHashMap<PortRefKey, FxHashSet<PortRefKey>>,
}

struct PortSnapshot {
    side: PortSide,
    node: Option<LNodeRef>,
    incoming_first: Option<LEdgeRef>,
    incoming_len: usize,
    outgoing_first: Option<LEdgeRef>,
    outgoing_len: usize,
    long_edge_target_node: Option<LNodeRef>,
    has_model_order: bool,
}

impl ModelOrderPortComparator {
    pub fn new(
        graph: LGraphRef,
        previous_layer: Vec<LNodeRef>,
        strategy: OrderingStrategy,
        target_node_model_order: Option<FxHashMap<NodeRefKey, i32>>,
        port_model_order: bool,
    ) -> Self {
        let mut previous_layer_position = FxHashMap::with_capacity_and_hasher(previous_layer.len(), Default::default());
        refill_layer_position_map(&mut previous_layer_position, &previous_layer);
        ModelOrderPortComparator {
            target_node_model_order: to_target_node_position_map(target_node_model_order),
            port_model_order,
            previous_layer,
            previous_layer_position,
            graph,
            strategy,
            bigger_than: FxHashMap::default(),
            smaller_than: FxHashMap::default(),
        }
    }

    pub fn compare(&mut self, original_p1: &LPortRef, original_p2: &LPortRef) -> i32 {
        let p1 = original_p1;
        let p2 = original_p2;
        let p1_key = PortRefKey(p1.clone());
        let p2_key = PortRefKey(p2.clone());
        self.ensure_sets(&p1_key);
        self.ensure_sets(&p2_key);
        if self
            .bigger_than
            .get(&p1_key)
            .is_some_and(|set| set.contains(&p2_key))
        {
            return 1;
        }
        if self
            .bigger_than
            .get(&p2_key)
            .is_some_and(|set| set.contains(&p1_key))
        {
            return -1;
        }
        if self
            .smaller_than
            .get(&p1_key)
            .is_some_and(|set| set.contains(&p2_key))
        {
            return -1;
        }
        if self
            .smaller_than
            .get(&p2_key)
            .is_some_and(|set| set.contains(&p1_key))
        {
            return 1;
        }

        let p1_snapshot = port_snapshot(p1);
        let p2_snapshot = port_snapshot(p2);
        let side1 = p1_snapshot.side;
        let side2 = p2_snapshot.side;
        if side1 != side2 {
            let result = match side1.cmp(&side2) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            if result > 0 {
                self.update_bigger_and_smaller(p1, p2, 1);
            } else {
                self.update_bigger_and_smaller(p2, p1, 1);
            }
            return result;
        }

        let mut reverse_order = 1;
        if p1_snapshot.incoming_first.is_some() && p2_snapshot.incoming_first.is_some() {
            if matches!(side1, PortSide::West | PortSide::North | PortSide::South) {
                reverse_order = -reverse_order;
            }

            let p1_source_port =
                p1_snapshot
                    .incoming_first
                    .as_ref()
                    .map(|edge| edge.lock().source())
                    .flatten();
            let p2_source_port =
                p2_snapshot
                    .incoming_first
                    .as_ref()
                    .map(|edge| edge.lock().source())
                    .flatten();
            if let (Some(p1_source_port), Some(p2_source_port)) = (p1_source_port, p2_source_port) {
                let p1_node = p1_source_port.lock().node();
                let p2_node = p2_source_port.lock().node();
                if let (Some(p1_node), Some(p2_node)) = (p1_node, p2_node) {
                    if std::sync::Arc::ptr_eq(&p1_node, &p2_node) {
                        let ports = p1_node.lock().ports().clone();
                        for port in ports {
                            if std::sync::Arc::ptr_eq(&port, &p1_source_port) {
                                self.update_bigger_and_smaller(p2, p1, reverse_order);
                                return -reverse_order;
                            } else if std::sync::Arc::ptr_eq(&port, &p2_source_port) {
                                self.update_bigger_and_smaller(p1, p2, reverse_order);
                                return reverse_order;
                            }
                        }
                    }

                    let p1_node_type = node_type(&p1_node);
                    let p2_node_type = node_type(&p2_node);
                    if p1_node_type
                        == crate::org::eclipse::elk::alg::layered::graph::NodeType::LongEdge
                        && p2_node_type
                            == crate::org::eclipse::elk::alg::layered::graph::NodeType::LongEdge
                        && layer_id(&p1_node) == layer_id(&p2_node)
                        && layer_id(&p1_node)
                            == layer_id(p1_snapshot.node.as_ref().unwrap_or(&p1_node))
                    {
                        let in_previous = {
                            let layer_opt = p1_node.lock().layer();
                            if let Some(layer) = layer_opt {
                                let layer_guard = layer.lock();
                                self.check_reference_layer(
                                    layer_guard.nodes(),
                                    &p1_node,
                                    &p2_node,
                                )
                            } else {
                                self.check_reference_layer(
                                    &self.previous_layer,
                                    &p1_node,
                                    &p2_node,
                                )
                            }
                        };
                        if in_previous != 0 {
                            if side1 == PortSide::East {
                                reverse_order = -reverse_order;
                            }
                            if in_previous > 0 {
                                self.update_bigger_and_smaller(p1, p2, reverse_order);
                                return reverse_order;
                            } else {
                                self.update_bigger_and_smaller(p2, p1, reverse_order);
                                return -reverse_order;
                            }
                        }
                    }

                    let in_previous = self.compare_in_previous_layer(&p1_node, &p2_node);
                    if in_previous != 0 {
                        if in_previous > 0 {
                            self.update_bigger_and_smaller(p1, p2, reverse_order);
                            return reverse_order;
                        } else {
                            self.update_bigger_and_smaller(p2, p1, reverse_order);
                            return -reverse_order;
                        }
                    }

                    if self.port_model_order {
                        let result = self.check_port_model_order(
                            p1,
                            p2,
                            p1_snapshot.has_model_order,
                            p2_snapshot.has_model_order,
                        );
                        if result != 0 {
                            if result > 0 {
                                self.update_bigger_and_smaller(p1, p2, reverse_order);
                                return reverse_order;
                            } else {
                                self.update_bigger_and_smaller(p2, p1, reverse_order);
                                return -reverse_order;
                            }
                        }
                    }
                }
            }
        }

        if p1_snapshot.outgoing_first.is_some() && p2_snapshot.outgoing_first.is_some() {
            if matches!(side1, PortSide::West | PortSide::South) {
                reverse_order = -reverse_order;
            }
            let p1_target_node = p1_snapshot.long_edge_target_node.clone();
            let p2_target_node = p2_snapshot.long_edge_target_node.clone();

            if self.strategy == OrderingStrategy::PreferNodes {
                if let (Some(p1_target_node), Some(p2_target_node)) =
                    (&p1_target_node, &p2_target_node)
                {
                    if has_model_order(p1_target_node) && has_model_order(p2_target_node) {
                        let max_nodes = match self.graph.try_lock() {            Some(graph_guard) => graph_guard
                                .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                                .unwrap_or(0),
            None => {
                                if ElkTrace::global().crossmin {
                                    eprintln!(
                                        "port_compare: graph lock busy, using default max_nodes"
                                    );
                                }
                                0
                            }
                        };
                        let p1_order =
                            CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                                p1_target_node,
                                p2_target_node,
                                &self.graph,
                                max_nodes,
                            );
                        let p2_order =
                            CMGroupModelOrderCalculator::calculate_model_order_or_group_model_order(
                                p2_target_node,
                                p1_target_node,
                                &self.graph,
                                max_nodes,
                            );
                        if p1_order > p2_order {
                            self.update_bigger_and_smaller(p1, p2, reverse_order);
                            return reverse_order;
                        } else {
                            self.update_bigger_and_smaller(p2, p1, reverse_order);
                            return -reverse_order;
                        }
                    }
                }
            }

            if self.port_model_order {
                let result = self.check_port_model_order(
                    p1,
                    p2,
                    p1_snapshot.has_model_order,
                    p2_snapshot.has_model_order,
                );
                if result != 0 {
                    if result > 0 {
                        self.update_bigger_and_smaller(p1, p2, reverse_order);
                        return reverse_order;
                    } else {
                        self.update_bigger_and_smaller(p2, p1, reverse_order);
                        return -reverse_order;
                    }
                }
            }

            let mut p1_order = edge_model_order(
                p1_snapshot.outgoing_first.as_ref().unwrap_or_else(|| unreachable!()),
                &self.graph,
                p1_snapshot.outgoing_len + p1_snapshot.incoming_len,
            );
            let mut p2_order = edge_model_order(
                p2_snapshot.outgoing_first.as_ref().unwrap_or_else(|| unreachable!()),
                &self.graph,
                p2_snapshot.outgoing_len + p2_snapshot.incoming_len,
            );

            if let (Some(p1_target_node), Some(p2_target_node)) = (&p1_target_node, &p2_target_node)
            {
                if std::sync::Arc::ptr_eq(p1_target_node, p2_target_node) {
                    if p1_order > p2_order {
                        self.update_bigger_and_smaller(p1, p2, reverse_order);
                        return reverse_order;
                    } else {
                        self.update_bigger_and_smaller(p2, p1, reverse_order);
                        return -reverse_order;
                    }
                }
                if let Some(map) = &self.target_node_model_order {
                    if let Some(order) = map.get(&node_ptr(p1_target_node)) {
                        p1_order = *order;
                    }
                    if let Some(order) = map.get(&node_ptr(p2_target_node)) {
                        p2_order = *order;
                    }
                }
            }

            if p1_order > p2_order {
                self.update_bigger_and_smaller(p1, p2, reverse_order);
                return reverse_order;
            } else {
                self.update_bigger_and_smaller(p2, p1, reverse_order);
                return -reverse_order;
            }
        }

        if p1_snapshot.incoming_first.is_some() && p2_snapshot.outgoing_first.is_some() {
            self.update_bigger_and_smaller(p1, p2, reverse_order);
            return 1;
        } else if p1_snapshot.outgoing_first.is_some() && p2_snapshot.incoming_first.is_some() {
            self.update_bigger_and_smaller(p2, p1, reverse_order);
            return -1;
        } else if p1_snapshot.has_model_order && p2_snapshot.has_model_order {
            let number_of_ports = p1_snapshot
                .node
                .as_ref()
                .map(|node| node.lock().ports().len())
                .unwrap_or(0) as i32;
            let p1_order = port_model_order(p1, &self.graph, number_of_ports);
            let p2_order = port_model_order(p2, &self.graph, number_of_ports);
            if matches!(side1, PortSide::West | PortSide::South) {
                reverse_order = -reverse_order;
            }
            if p1_order > p2_order {
                self.update_bigger_and_smaller(p1, p2, reverse_order);
                return reverse_order;
            } else {
                self.update_bigger_and_smaller(p2, p1, reverse_order);
                return -reverse_order;
            }
        }

        self.update_bigger_and_smaller(p2, p1, reverse_order);
        -reverse_order
    }

    pub fn clear_transitive_ordering(&mut self) {
        self.bigger_than.clear();
        self.smaller_than.clear();
    }

    pub fn reset_for_previous_layer(&mut self, previous_layer: Vec<LNodeRef>) {
        self.previous_layer = previous_layer;
        refill_layer_position_map(&mut self.previous_layer_position, &self.previous_layer);
        self.target_node_model_order = None;
        self.clear_transitive_ordering();
    }

    pub fn reset_for_previous_layer_slice(&mut self, previous_layer: &[LNodeRef]) {
        self.previous_layer.clear();
        self.previous_layer.extend(previous_layer.iter().cloned());
        refill_layer_position_map(&mut self.previous_layer_position, &self.previous_layer);
        self.target_node_model_order = None;
        self.clear_transitive_ordering();
    }

    pub fn reset_for_node_target_model_order(
        &mut self,
        target_node_model_order: Option<FxHashMap<NodeRefKey, i32>>,
    ) {
        self.target_node_model_order = to_target_node_position_map(target_node_model_order);
        self.clear_transitive_ordering();
    }

    fn check_port_model_order(
        &self,
        p1: &LPortRef,
        p2: &LPortRef,
        p1_has_model_order: bool,
        p2_has_model_order: bool,
    ) -> i32 {
        if p1_has_model_order && p2_has_model_order {
            let number_of_ports = port_node(p1)
                .map(|node| node.lock().ports().len())
                .unwrap_or(0) as i32;
            let p1_order = port_model_order(p1, &self.graph, number_of_ports);
            let p2_order = port_model_order(p2, &self.graph, number_of_ports);
            return match p1_order.cmp(&p2_order) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
        }
        0
    }

    fn update_bigger_and_smaller(
        &mut self,
        bigger_ori: &LPortRef,
        smaller_ori: &LPortRef,
        reverse_order: i32,
    ) {
        let (bigger, smaller) = if reverse_order < 0 {
            (smaller_ori, bigger_ori)
        } else {
            (bigger_ori, smaller_ori)
        };
        let bigger_key = PortRefKey(bigger.clone());
        let smaller_key = PortRefKey(smaller.clone());
        self.ensure_sets(&bigger_key);
        self.ensure_sets(&smaller_key);

        let smaller_bigger = self
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

        for very_small in smaller_bigger.iter() {
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
                .extend(smaller_bigger.iter().cloned());
        }
    }

    fn ensure_sets(&mut self, key: &PortRefKey) {
        self.bigger_than.entry(key.clone()).or_default();
        self.smaller_than.entry(key.clone()).or_default();
    }

    fn check_reference_layer(
        &self,
        layer: &[LNodeRef],
        p1_node: &LNodeRef,
        p2_node: &LNodeRef,
    ) -> i32 {
        for node in layer {
            if std::sync::Arc::ptr_eq(node, p1_node) {
                return -1;
            } else if std::sync::Arc::ptr_eq(node, p2_node) {
                return 1;
            }
        }
        0
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
}

fn port_node(port: &LPortRef) -> Option<LNodeRef> {
    port.lock().node()
}

fn node_type(node: &LNodeRef) -> crate::org::eclipse::elk::alg::layered::graph::NodeType {
    node.lock().node_type()
}

fn layer_id(node: &LNodeRef) -> usize {
    let layer_opt = node.lock().layer();
    if let Some(layer) = layer_opt {
        layer.lock().graph_element().id as usize
    } else {
        0
    }
}

fn has_model_order(node: &LNodeRef) -> bool {
    node.lock()
        .get_property(InternalProperties::MODEL_ORDER)
        .is_some()
}

fn port_model_order(port: &LPortRef, graph: &LGraphRef, offset: i32) -> i32 {
    let order = port
        .lock()
        .get_property(InternalProperties::MODEL_ORDER);
    let Some(order) = order else {
        return -1;
    };
    let enforce_group_model_order = graph
        .try_lock()

        .and_then(|graph_guard| {
            graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
        })
        .unwrap_or(
            crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::OnlyWithinGroup,
        )
        == crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::Enforced;
    if enforce_group_model_order {
        let enforced_orders = graph
            .try_lock()

            .and_then(|graph_guard| {
                graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
            })
            .unwrap_or_default();
        let group_id = port
            .lock()
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
            .unwrap_or(0);
        if enforced_orders.contains(&group_id) {
            return offset * group_id + order;
        }
    }
    order
}

fn edge_model_order(edge: &LEdgeRef, graph: &LGraphRef, offset: usize) -> i32 {
    let order = edge
        .lock()
        .get_property(InternalProperties::MODEL_ORDER);
    let Some(order) = order else {
        return 0;
    };
    let enforce_group_model_order = graph
        .try_lock()

        .and_then(|graph_guard| {
            graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
        })
        .unwrap_or(
            crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::OnlyWithinGroup,
        )
        == crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::Enforced;
    if enforce_group_model_order {
        let enforced_orders = graph
            .try_lock()

            .and_then(|graph_guard| {
                graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
            })
            .unwrap_or_default();
        let group_id = edge
            .lock()
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
            .unwrap_or(0);
        if enforced_orders.contains(&group_id) {
            return (offset as i32) * group_id + order;
        }
    }
    order
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

fn to_target_node_position_map(
    target_node_model_order: Option<FxHashMap<NodeRefKey, i32>>,
) -> Option<FxHashMap<usize, i32>> {
    target_node_model_order.map(|orders| {
        let mut pointer_orders = FxHashMap::with_capacity_and_hasher(orders.len(), Default::default());
        for (key, value) in orders {
            pointer_orders.insert(node_ptr(&key.0), value);
        }
        pointer_orders
    })
}

fn port_snapshot(port: &LPortRef) -> PortSnapshot {
    let port_guard = port.lock();
    let (incoming_first, incoming_len) = {
        let incoming = port_guard.incoming_edges();
        (incoming.first().cloned(), incoming.len())
    };
    let (outgoing_first, outgoing_len) = {
        let outgoing = port_guard.outgoing_edges();
        (outgoing.first().cloned(), outgoing.len())
    };
    PortSnapshot {
        side: port_guard.side(),
        node: port_guard.node(),
        incoming_first,
        incoming_len,
        outgoing_first,
        outgoing_len,
        long_edge_target_node: port_guard.get_property(InternalProperties::LONG_EDGE_TARGET_NODE),
        has_model_order: port_guard
            .get_property(InternalProperties::MODEL_ORDER)
            .is_some(),
    }
}
