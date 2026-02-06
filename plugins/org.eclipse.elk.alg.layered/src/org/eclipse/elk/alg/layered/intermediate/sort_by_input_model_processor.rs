#![allow(clippy::mutable_key_type)]

use std::collections::HashMap;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeRefKey, NodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct SortByInputModelProcessor;

impl SortByInputModelProcessor {
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
