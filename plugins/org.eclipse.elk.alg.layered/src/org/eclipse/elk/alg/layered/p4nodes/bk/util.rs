use std::collections::HashMap;
use std::sync::Arc;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};

use super::aligned_layout::BKAlignedLayout;

pub(crate) fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

pub(crate) fn node_type(node: &LNodeRef) -> NodeType {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(NodeType::Normal)
}

pub(crate) fn node_margin_top(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.margin().top)
        .unwrap_or(0.0)
}

pub(crate) fn node_margin_bottom(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.margin().bottom)
        .unwrap_or(0.0)
}

pub(crate) fn node_size_y(node: &LNodeRef) -> f64 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().size_ref().y)
        .unwrap_or(0.0)
}

pub(crate) fn node_to_string(node: &LNodeRef) -> String {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.to_string())
        .unwrap_or_else(|| "n_".to_string())
}

pub(crate) fn port_offset_y(port: &LPortRef) -> f64 {
    port.lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().position_ref().y + port_guard.anchor_ref().y)
        .unwrap_or(0.0)
}

pub(crate) fn port_node_id(port: &LPortRef) -> usize {
    port.lock()
        .ok()
        .and_then(|port_guard| port_guard.node())
        .map(|node| node_id(&node))
        .unwrap_or(0)
}

pub(crate) fn edge_key(edge: &LEdgeRef) -> usize {
    Arc::as_ptr(edge) as usize
}

pub(crate) fn edge_between(source: &LNodeRef, target: &LNodeRef) -> Option<LEdgeRef> {
    let edges = source
        .lock()
        .ok()
        .map(|node_guard| node_guard.connected_edges())
        .unwrap_or_default();
    let target_id = node_id(target);

    for edge in edges {
        let other = edge
            .lock()
            .ok()
            .map(|edge_guard| edge_guard.other_node(source));
        if let Some(other) = other {
            if node_id(&other) == target_id {
                return Some(edge);
            }
        }
    }

    None
}

pub(crate) fn get_blocks(bal: &BKAlignedLayout) -> HashMap<usize, Vec<LNodeRef>> {
    let mut blocks: HashMap<usize, Vec<LNodeRef>> = HashMap::new();

    for layer in &bal.layers {
        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            let id = node_id(&node);
            let root_id = bal.root[id];
            blocks.entry(root_id).or_default().push(node);
        }
    }

    blocks
}
