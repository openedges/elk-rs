use std::collections::BTreeMap;
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
    fn edge_matches(edge: &LEdgeRef, source_id: usize, target_id: usize) -> bool {
        let (src_node, tgt_node) = edge
            .lock()
            .ok()
            .map(|edge_guard| {
                let src = edge_guard
                    .source()
                    .and_then(|port| port.lock().ok().and_then(|port| port.node()));
                let tgt = edge_guard
                    .target()
                    .and_then(|port| port.lock().ok().and_then(|port| port.node()));
                (src, tgt)
            })
            .unwrap_or((None, None));
        if let (Some(src_node), Some(tgt_node)) = (src_node, tgt_node) {
            let src_id = node_id(&src_node);
            let tgt_id = node_id(&tgt_node);
            return (src_id == source_id && tgt_id == target_id)
                || (src_id == target_id && tgt_id == source_id);
        }
        false
    }

    let source_id = node_id(source);
    let target_id = node_id(target);

    let source_edges = source
        .lock()
        .ok()
        .map(|node_guard| node_guard.connected_edges())
        .unwrap_or_default();
    if let Some(edge) = source_edges
        .into_iter()
        .find(|edge| edge_matches(edge, source_id, target_id))
    {
        return Some(edge);
    }

    let target_edges = target
        .lock()
        .ok()
        .map(|node_guard| node_guard.connected_edges())
        .unwrap_or_default();
    target_edges
        .into_iter()
        .find(|edge| edge_matches(edge, source_id, target_id))
}

pub(crate) fn get_blocks(bal: &BKAlignedLayout) -> BTreeMap<usize, Vec<LNodeRef>> {
    let mut blocks: BTreeMap<usize, Vec<LNodeRef>> = BTreeMap::new();

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
