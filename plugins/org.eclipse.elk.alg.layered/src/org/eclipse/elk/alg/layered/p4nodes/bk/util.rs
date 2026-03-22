use std::collections::BTreeMap;
use std::sync::Arc;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};

use super::aligned_layout::BKAlignedLayout;

pub(crate) fn node_id(node: &LNodeRef) -> usize {
    node.lock().shape().graph_element().id as usize
}

pub(crate) fn node_type(node: &LNodeRef) -> NodeType {
    node.lock().node_type()
}

pub(crate) fn node_margin_top(node: &LNodeRef) -> f64 {
    node.lock().margin().top
}

pub(crate) fn node_margin_bottom(node: &LNodeRef) -> f64 {
    node.lock().margin().bottom
}

pub(crate) fn node_size_y(node: &LNodeRef) -> f64 {
    node.lock().shape().size_ref().y
}

pub(crate) fn node_to_string(node: &LNodeRef) -> String {
    node.lock().to_string()
}

pub(crate) fn port_offset_y(port: &LPortRef) -> f64 {
    let mut port_guard = port.lock();
    port_guard.shape().position_ref().y + port_guard.anchor_ref().y
}

pub(crate) fn port_node_id(port: &LPortRef) -> usize {
    port.lock().node()
        .map(|node| node_id(&node))
        .unwrap_or(0)
}

pub(crate) fn edge_key(edge: &LEdgeRef) -> usize {
    Arc::as_ptr(edge) as usize
}

pub(crate) fn edge_between(source: &LNodeRef, target: &LNodeRef) -> Option<LEdgeRef> {
    fn edge_matches(edge: &LEdgeRef, source_id: usize, target_id: usize) -> bool {
        let (src_node, tgt_node) = {
            let edge_guard = edge.lock();
            let src = edge_guard
                .source()
                .and_then(|port| port.lock().node());
            let tgt = edge_guard
                .target()
                .and_then(|port| port.lock().node());
            (src, tgt)
        };
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
        .lock().connected_edges();
    if let Some(edge) = source_edges
        .into_iter()
        .find(|edge| edge_matches(edge, source_id, target_id))
    {
        return Some(edge);
    }

    let target_edges = target
        .lock().connected_edges();
    target_edges
        .into_iter()
        .find(|edge| edge_matches(edge, source_id, target_id))
}

pub(crate) fn get_blocks(bal: &BKAlignedLayout) -> BTreeMap<usize, Vec<LNodeRef>> {
    let mut blocks: BTreeMap<usize, Vec<LNodeRef>> = BTreeMap::new();

    for layer in &bal.layers {
        let nodes = layer
            .lock().nodes().clone();
        for node in nodes {
            let id = node_id(&node);
            let root_id = bal.root[id];
            blocks.entry(root_id).or_default().push(node);
        }
    }

    blocks
}
