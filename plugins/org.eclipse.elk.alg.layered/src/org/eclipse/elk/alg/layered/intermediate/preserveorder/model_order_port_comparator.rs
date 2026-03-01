#![allow(clippy::mutable_key_type)]

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraphRef, LNodeRef, LPortRef, NodeRefKey,
};
use crate::org::eclipse::elk::alg::layered::intermediate::CMGroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, OrderingStrategy,
};

static TRACE_CROSSMIN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());

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
    target_node_model_order: Option<HashMap<NodeRefKey, i32>>,
    port_model_order: bool,
    previous_layer: Vec<LNodeRef>,
    previous_layer_position: HashMap<NodeRefKey, usize>,
    graph: LGraphRef,
    strategy: OrderingStrategy,
    bigger_than: HashMap<PortRefKey, HashSet<PortRefKey>>,
    smaller_than: HashMap<PortRefKey, HashSet<PortRefKey>>,
}

impl ModelOrderPortComparator {
    pub fn new(
        graph: LGraphRef,
        previous_layer: Vec<LNodeRef>,
        strategy: OrderingStrategy,
        target_node_model_order: Option<HashMap<NodeRefKey, i32>>,
        port_model_order: bool,
    ) -> Self {
        let previous_layer_position = build_layer_position_map(&previous_layer);
        ModelOrderPortComparator {
            target_node_model_order,
            port_model_order,
            previous_layer,
            previous_layer_position,
            graph,
            strategy,
            bigger_than: HashMap::new(),
            smaller_than: HashMap::new(),
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

        let side1 = port_side(p1);
        let side2 = port_side(p2);
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
        let incoming1 = incoming_edges(p1);
        let incoming2 = incoming_edges(p2);
        if !incoming1.is_empty() && !incoming2.is_empty() {
            if matches!(side1, PortSide::West | PortSide::North | PortSide::South) {
                reverse_order = -reverse_order;
            }

            let p1_source_port = incoming1[0]
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source());
            let p2_source_port = incoming2[0]
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source());
            if let (Some(p1_source_port), Some(p2_source_port)) = (p1_source_port, p2_source_port) {
                let p1_node = p1_source_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                let p2_node = p2_source_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                if let (Some(p1_node), Some(p2_node)) = (p1_node, p2_node) {
                    if std::sync::Arc::ptr_eq(&p1_node, &p2_node) {
                        let ports = p1_node
                            .lock()
                            .ok()
                            .map(|node_guard| node_guard.ports().clone())
                            .unwrap_or_default();
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
                            == layer_id(&port_node(p1).unwrap_or_else(|| p1_node.clone()))
                    {
                        let reference_layer = p1_node
                            .lock()
                            .ok()
                            .and_then(|node_guard| node_guard.layer())
                            .and_then(|layer| {
                                layer
                                    .lock()
                                    .ok()
                                    .map(|layer_guard| layer_guard.nodes().clone())
                            })
                            .unwrap_or_else(|| self.previous_layer.clone());
                        let in_previous =
                            self.check_reference_layer(&reference_layer, &p1_node, &p2_node);
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
                        let result = self.check_port_model_order(p1, p2);
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

        let outgoing1 = outgoing_edges(p1);
        let outgoing2 = outgoing_edges(p2);
        if !outgoing1.is_empty() && !outgoing2.is_empty() {
            if matches!(side1, PortSide::West | PortSide::South) {
                reverse_order = -reverse_order;
            }
            let p1_target_node = p1.lock().ok().and_then(|mut port_guard| {
                port_guard.get_property(InternalProperties::LONG_EDGE_TARGET_NODE)
            });
            let p2_target_node = p2.lock().ok().and_then(|mut port_guard| {
                port_guard.get_property(InternalProperties::LONG_EDGE_TARGET_NODE)
            });

            if self.strategy == OrderingStrategy::PreferNodes {
                if let (Some(p1_target_node), Some(p2_target_node)) =
                    (&p1_target_node, &p2_target_node)
                {
                    if has_model_order(p1_target_node) && has_model_order(p2_target_node) {
                        let max_nodes = match self.graph.try_lock() {
                            Ok(mut graph_guard) => graph_guard
                                .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                                .unwrap_or(0),
                            Err(_) => {
                                if *TRACE_CROSSMIN {
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
                let result = self.check_port_model_order(p1, p2);
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
                &outgoing1[0],
                &self.graph,
                outgoing1.len() + incoming1.len(),
            );
            let mut p2_order = edge_model_order(
                &outgoing2[0],
                &self.graph,
                outgoing2.len() + incoming2.len(),
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
                    if let Some(order) = map.get(&NodeRefKey(p1_target_node.clone())) {
                        p1_order = *order;
                    }
                    if let Some(order) = map.get(&NodeRefKey(p2_target_node.clone())) {
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

        if !incoming1.is_empty() && !outgoing2.is_empty() {
            self.update_bigger_and_smaller(p1, p2, reverse_order);
            return 1;
        } else if !outgoing1.is_empty() && !incoming2.is_empty() {
            self.update_bigger_and_smaller(p2, p1, reverse_order);
            return -1;
        } else if has_port_model_order(p1) && has_port_model_order(p2) {
            let number_of_ports = port_node(p1)
                .and_then(|node| node.lock().ok().map(|node_guard| node_guard.ports().len()))
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
        self.previous_layer_position = build_layer_position_map(&self.previous_layer);
        self.target_node_model_order = None;
        self.clear_transitive_ordering();
    }

    pub fn reset_for_previous_layer_slice(&mut self, previous_layer: &[LNodeRef]) {
        self.previous_layer.clear();
        self.previous_layer.extend(previous_layer.iter().cloned());
        self.previous_layer_position = build_layer_position_map(&self.previous_layer);
        self.target_node_model_order = None;
        self.clear_transitive_ordering();
    }

    pub fn reset_for_node_target_model_order(
        &mut self,
        target_node_model_order: Option<HashMap<NodeRefKey, i32>>,
    ) {
        self.target_node_model_order = target_node_model_order;
        self.clear_transitive_ordering();
    }

    fn check_port_model_order(&self, p1: &LPortRef, p2: &LPortRef) -> i32 {
        if has_port_model_order(p1) && has_port_model_order(p2) {
            let number_of_ports = port_node(p1)
                .and_then(|node| node.lock().ok().map(|node_guard| node_guard.ports().len()))
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
        let p1_pos = self
            .previous_layer_position
            .get(&NodeRefKey(p1_node.clone()))
            .copied();
        let p2_pos = self
            .previous_layer_position
            .get(&NodeRefKey(p2_node.clone()))
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
}

fn incoming_edges(port: &LPortRef) -> Vec<LEdgeRef> {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.incoming_edges().clone())
        .unwrap_or_default()
}

fn outgoing_edges(port: &LPortRef) -> Vec<LEdgeRef> {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.outgoing_edges().clone())
        .unwrap_or_default()
}

fn port_side(port: &LPortRef) -> PortSide {
    port.lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined)
}

fn port_node(port: &LPortRef) -> Option<LNodeRef> {
    port.lock().ok().and_then(|port_guard| port_guard.node())
}

fn node_type(node: &LNodeRef) -> crate::org::eclipse::elk::alg::layered::graph::NodeType {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(crate::org::eclipse::elk::alg::layered::graph::NodeType::Normal)
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

fn has_model_order(node: &LNodeRef) -> bool {
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::MODEL_ORDER))
        .is_some()
}

fn has_port_model_order(port: &LPortRef) -> bool {
    port.lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::MODEL_ORDER))
        .is_some()
}

fn port_model_order(port: &LPortRef, graph: &LGraphRef, offset: i32) -> i32 {
    let order = port
        .lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::MODEL_ORDER));
    let Some(order) = order else {
        return -1;
    };
    let enforce_group_model_order = graph
        .try_lock()
        .ok()
        .and_then(|mut graph_guard| {
            graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
        })
        .unwrap_or(
            crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::OnlyWithinGroup,
        )
        == crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::Enforced;
    if enforce_group_model_order {
        let enforced_orders = graph
            .try_lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
            })
            .unwrap_or_default();
        let group_id = port
            .lock()
            .ok()
            .and_then(|mut port_guard| {
                port_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
            })
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
        .ok()
        .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::MODEL_ORDER));
    let Some(order) = order else {
        return 0;
    };
    let enforce_group_model_order = graph
        .try_lock()
        .ok()
        .and_then(|mut graph_guard| {
            graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
        })
        .unwrap_or(
            crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::OnlyWithinGroup,
        )
        == crate::org::eclipse::elk::alg::layered::options::GroupOrderStrategy::Enforced;
    if enforce_group_model_order {
        let enforced_orders = graph
            .try_lock()
            .ok()
            .and_then(|mut graph_guard| {
                graph_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
            })
            .unwrap_or_default();
        let group_id = edge
            .lock()
            .ok()
            .and_then(|mut edge_guard| {
                edge_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
            })
            .unwrap_or(0);
        if enforced_orders.contains(&group_id) {
            return (offset as i32) * group_id + order;
        }
    }
    order
}

fn build_layer_position_map(layer: &[LNodeRef]) -> HashMap<NodeRefKey, usize> {
    let mut positions = HashMap::with_capacity(layer.len());
    for (index, node) in layer.iter().enumerate() {
        positions.insert(NodeRefKey(node.clone()), index);
    }
    positions
}
