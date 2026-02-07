#![allow(clippy::mutable_key_type)]

use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeRefKey, NodeType};
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef};
use crate::org::eclipse::elk::alg::layered::intermediate::preserveorder::{
    ModelOrderNodeComparator, ModelOrderPortComparator,
};
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions, LongEdgeOrderingStrategy,
    OrderingStrategy,
};

pub struct SortByInputModelProcessor;

impl ILayoutProcessor<LGraph> for SortByInputModelProcessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Sort By Input Model", 1.0);

        let layers = graph.layers().clone();
        if layers.is_empty() {
            progress_monitor.done();
            return;
        }

        let ordering_strategy = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY)
            .unwrap_or(OrderingStrategy::None);
        let long_edge_strategy = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY)
            .unwrap_or(LongEdgeOrderingStrategy::Equal);
        let group_strategy = graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup);
        let port_model_order = graph
            .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER)
            .unwrap_or(false);
        let comparator_graph = build_ordering_context_graph(graph);

        for (layer_index, layer) in layers.iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_index as i32;
            }

            let previous_layer_index = if layer_index == 0 { 0 } else { layer_index - 1 };
            let previous_layer_nodes = layers
                .get(previous_layer_index)
                .and_then(|prev| prev.lock().ok().map(|layer_guard| layer_guard.nodes().clone()))
                .unwrap_or_default();

            let mut node_comparator = ModelOrderNodeComparator::new(
                comparator_graph.clone(),
                previous_layer_nodes.clone(),
                ordering_strategy,
                long_edge_strategy,
                group_strategy,
                true,
            );
            let mut nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            Self::insertion_sort(nodes.as_mut_slice(), &mut node_comparator);
            if let Ok(mut layer_guard) = layer.lock() {
                *layer_guard.nodes_mut() = nodes;
            }

            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let constraints = node
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| {
                        node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS)
                    })
                    .unwrap_or(PortConstraints::Undefined);
                if matches!(constraints, PortConstraints::FixedOrder | PortConstraints::FixedPos) {
                    continue;
                }

                let long_edge_targets = Self::long_edge_target_node_preprocessing(&node);
                let mut port_comparator = ModelOrderPortComparator::new(
                    comparator_graph.clone(),
                    previous_layer_nodes.clone(),
                    ordering_strategy,
                    Some(long_edge_targets),
                    port_model_order,
                );
                let mut ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                Self::insertion_sort_port(ports.as_mut_slice(), &mut port_comparator);
                if let Ok(mut node_guard) = node.lock() {
                    *node_guard.ports_mut() = ports;
                    node_guard.cache_port_sides();
                }
            }

            let mut node_comparator = ModelOrderNodeComparator::new(
                comparator_graph.clone(),
                previous_layer_nodes,
                ordering_strategy,
                long_edge_strategy,
                group_strategy,
                false,
            );
            let mut nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            Self::insertion_sort(nodes.as_mut_slice(), &mut node_comparator);
            if let Ok(mut layer_guard) = layer.lock() {
                *layer_guard.nodes_mut() = nodes;
            }
        }

        progress_monitor.done();
    }
}

impl SortByInputModelProcessor {
    pub fn insertion_sort(layer: &mut [LNodeRef], comparator: &mut ModelOrderNodeComparator) {
        for i in 1..layer.len() {
            let temp = layer[i].clone();
            let mut j = i;
            while j > 0 && comparator.compare(&layer[j - 1], &temp) > 0 {
                layer[j] = layer[j - 1].clone();
                j -= 1;
            }
            layer[j] = temp;
        }
        comparator.clear_transitive_ordering();
    }

    pub fn insertion_sort_port(layer: &mut [LPortRef], comparator: &mut ModelOrderPortComparator) {
        for i in 1..layer.len() {
            let temp = layer[i].clone();
            let mut j = i;
            while j > 0 && comparator.compare(&layer[j - 1], &temp) > 0 {
                layer[j] = layer[j - 1].clone();
                j -= 1;
            }
            layer[j] = temp;
        }
        comparator.clear_transitive_ordering();
    }

    pub fn long_edge_target_node_preprocessing(node: &LNodeRef) -> HashMap<NodeRefKey, i32> {
        if let Ok(mut node_guard) = node.lock() {
            if let Some(existing) =
                node_guard.get_property(InternalProperties::TARGET_NODE_MODEL_ORDER)
            {
                return existing;
            }
        }

        let mut target_node_model_order: HashMap<NodeRefKey, i32> = HashMap::new();
        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();
        for port in ports {
            let outgoing = port
                .lock()
                .ok()
                .map(|port_guard| port_guard.outgoing_edges().clone())
                .unwrap_or_default();
            if outgoing.is_empty() {
                continue;
            }
            let target_node = get_target_node(&port);
            if let Some(target_node) = &target_node {
                if let Ok(mut port_guard) = port.lock() {
                    port_guard.set_property(
                        InternalProperties::LONG_EDGE_TARGET_NODE,
                        Some(target_node.clone()),
                    );
                }
                let prev_order = target_node_model_order
                    .get(&NodeRefKey(target_node.clone()))
                    .copied()
                    .unwrap_or(i32::MAX);
                let edge = outgoing.first().cloned();
                if let Some(edge) = edge {
                    let reversed = edge
                        .lock()
                        .ok()
                        .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::REVERSED))
                        .unwrap_or(false);
                    if !reversed {
                        let order = edge
                            .lock()
                            .ok()
                            .and_then(|mut edge_guard| {
                                edge_guard.get_property(InternalProperties::MODEL_ORDER)
                            })
                            .unwrap_or(i32::MAX);
                        target_node_model_order.insert(
                            NodeRefKey(target_node.clone()),
                            prev_order.min(order),
                        );
                    }
                }
            }
        }

        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_property(
                InternalProperties::TARGET_NODE_MODEL_ORDER,
                Some(target_node_model_order.clone()),
            );
        }
        target_node_model_order
    }
}

pub fn get_target_node(port: &LPortRef) -> Option<LNodeRef> {
    let mut edge = port
        .lock()
        .ok()
        .and_then(|port_guard| port_guard.outgoing_edges().first().cloned());
    while let Some(edge_ref) = edge.clone() {
        let target_node = edge_ref
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        let target_node = target_node?;
        if let Ok(mut node_guard) = target_node.lock() {
            if let Some(long_edge_target) =
                node_guard.get_property(InternalProperties::LONG_EDGE_TARGET)
            {
                if let Some(target) = long_edge_target
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node())
                {
                    return Some(target);
                }
            }
            if node_guard.node_type() != NodeType::Normal {
                let outgoing = node_guard.outgoing_edges();
                if let Some(next_edge) = outgoing.first() {
                    edge = Some(next_edge.clone());
                    continue;
                }
                return None;
            }
        }
        return Some(target_node);
    }
    None
}

fn build_ordering_context_graph(graph: &mut LGraph) -> LGraphRef {
    let context = LGraph::new();
    if let Ok(mut context_guard) = context.lock() {
        if let Some(max_model_order_nodes) = graph.get_property(InternalProperties::MAX_MODEL_ORDER_NODES) {
            context_guard.set_property(
                InternalProperties::MAX_MODEL_ORDER_NODES,
                Some(max_model_order_nodes),
            );
        }
        if let Some(group_order_strategy) =
            graph.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
        {
            context_guard.set_property(
                LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY,
                Some(group_order_strategy),
            );
        }
        if let Some(enforced_group_orders) =
            graph.get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
        {
            context_guard.set_property(
                LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS,
                Some(enforced_group_orders),
            );
        }
    }
    context
}
