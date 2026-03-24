#![allow(clippy::mutable_key_type)]

use rustc_hash::FxHashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LGraphRef};
use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeRefKey, NodeType};
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
        let mut pre_ports_node_comparator = ModelOrderNodeComparator::new(
            comparator_graph.clone(),
            Vec::new(),
            ordering_strategy,
            long_edge_strategy,
            group_strategy,
            true,
        );
        let mut post_ports_node_comparator = ModelOrderNodeComparator::new(
            comparator_graph.clone(),
            Vec::new(),
            ordering_strategy,
            long_edge_strategy,
            group_strategy,
            false,
        );
        let mut port_comparator = ModelOrderPortComparator::new(
            comparator_graph.clone(),
            Vec::new(),
            ordering_strategy,
            None,
            port_model_order,
        );

        let mut previous_layer_nodes = Vec::new();
        for (layer_index, layer) in layers.iter().enumerate() {
            let mut nodes = {
                let mut layer_guard = layer.lock();
                layer_guard.graph_element().id = layer_index as i32;
                layer_guard.nodes().clone()
            };
            // Java semantics: for the first layer, previousLayer is the layer itself.
            if layer_index == 0 {
                previous_layer_nodes = nodes.clone();
            }

            pre_ports_node_comparator.reset_for_previous_layer_slice(&previous_layer_nodes);
            Self::insertion_sort(nodes.as_mut_slice(), &mut pre_ports_node_comparator);
            {
                let mut layer_guard = layer.lock();
                layer_guard.nodes_mut().clone_from(&nodes);
            }

            port_comparator.reset_for_previous_layer_slice(&previous_layer_nodes);
            for node in &nodes {
                let constraints = node
                    .lock()
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined);
                if matches!(
                    constraints,
                    PortConstraints::FixedOrder | PortConstraints::FixedPos
                ) {
                    continue;
                }

                let long_edge_targets = Self::long_edge_target_node_preprocessing(node);
                port_comparator.reset_for_node_target_model_order(Some(long_edge_targets));
                let mut ports = node
                    .lock().ports().clone();
                Self::insertion_sort_port(ports.as_mut_slice(), &mut port_comparator);
                {
                    let mut node_guard = node.lock();
                    *node_guard.ports_mut() = ports;
                    node_guard.cache_port_sides();
                }
            }

            post_ports_node_comparator.reset_for_previous_layer_slice(&previous_layer_nodes);
            Self::insertion_sort(nodes.as_mut_slice(), &mut post_ports_node_comparator);
            {
                let mut layer_guard = layer.lock();
                layer_guard.nodes_mut().clone_from(&nodes);
            }
            previous_layer_nodes = nodes;
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

    pub fn long_edge_target_node_preprocessing(node: &LNodeRef) -> FxHashMap<NodeRefKey, i32> {
        let mut target_node_model_order: FxHashMap<NodeRefKey, i32> = FxHashMap::default();
        let ports = {
            let node_guard = node.lock();
            if let Some(existing) =
                node_guard.get_property(InternalProperties::TARGET_NODE_MODEL_ORDER)
            {
                return existing;
            }
            node_guard.ports().clone()
        };
        for port in ports {
            let first_edge = port.lock().outgoing_edges().first().cloned();
            let Some(first_edge) = first_edge else {
                continue;
            };

            let target_node = get_target_node_from_edge(first_edge.clone());
            if let Some(target_node) = &target_node {
                {
                    let mut port_guard = port.lock();
                    port_guard.set_property(
                        InternalProperties::LONG_EDGE_TARGET_NODE,
                        Some(target_node.clone()),
                    );
                }
                let target_node_key = NodeRefKey(target_node.clone());
                let prev_order = target_node_model_order
                    .get(&target_node_key)
                    .copied()
                    .unwrap_or(i32::MAX);
                let (reversed, model_order) = {
                    let edge_guard = first_edge.lock();
                    (
                        edge_guard
                            .get_property(InternalProperties::REVERSED)
                            .unwrap_or(false),
                        edge_guard
                            .get_property(InternalProperties::MODEL_ORDER)
                            .unwrap_or(i32::MAX),
                    )
                };
                if !reversed {
                    target_node_model_order.insert(target_node_key, prev_order.min(model_order));
                }
            }
        }

        {
            let mut node_guard = node.lock();
            node_guard.set_property(
                InternalProperties::TARGET_NODE_MODEL_ORDER,
                Some(target_node_model_order.clone()),
            );
        }
        target_node_model_order
    }
}

pub fn get_target_node(port: &LPortRef) -> Option<LNodeRef> {
    let edge = port.lock().outgoing_edges().first().cloned();
    edge.and_then(get_target_node_from_edge)
}

fn get_target_node_from_edge(mut edge: LEdgeRef) -> Option<LNodeRef> {
    loop {
        let target_node = edge
            .lock().target()
            .and_then(|port| port.lock().node())?;

        {
            let node_guard = target_node.lock();
            if let Some(long_edge_target) =
                node_guard.get_property(InternalProperties::LONG_EDGE_TARGET)
            {
                if let Some(target) = long_edge_target
                    .lock().node()
                {
                    return Some(target);
                }
            }

            if node_guard.node_type() != NodeType::Normal {
                if let Some(next_edge) = node_guard.outgoing_edges().first().cloned() {
                    edge = next_edge;
                    continue;
                }
                return None;
            }
        }

        return Some(target_node);
    }
}

fn build_ordering_context_graph(graph: &mut LGraph) -> LGraphRef {
    let context = LGraph::new();
    {
        let mut context_guard = context.lock();
        if let Some(max_model_order_nodes) =
            graph.get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
        {
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
