use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

#[derive(Default)]
pub struct NeighborhoodInformation {
    pub node_count: usize,
    pub layer_index: Vec<usize>,
    pub node_index: Vec<usize>,
    pub left_neighbors: Vec<Vec<Pair<LNodeRef, LEdgeRef>>>,
    pub right_neighbors: Vec<Vec<Pair<LNodeRef, LEdgeRef>>>,
}

impl NeighborhoodInformation {
    pub fn build_for(graph: &mut LGraph) -> Self {
        let mut ni = NeighborhoodInformation::default();

        let layers = graph.layers().clone();
        for layer in layers.iter() {
            if let Ok(layer_guard) = layer.lock() {
                ni.node_count += layer_guard.nodes().len();
            }
        }

        ni.layer_index = vec![0; layers.len()];
        ni.node_index = vec![0; ni.node_count];

        let mut layer_id = 0usize;
        let mut node_counter = 0usize;
        for (layer_idx, layer) in layers.iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_id as i32;
                ni.layer_index[layer_id] = layer_idx;
                layer_id += 1;
                let nodes = layer_guard.nodes().clone();
                for (local_index, node) in nodes.into_iter().enumerate() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.shape().graph_element().id = node_counter as i32;
                    }
                    if node_counter >= ni.node_index.len() {
                        ni.node_index.resize(node_counter + 1, 0);
                    }
                    ni.node_index[node_counter] = local_index;
                    node_counter += 1;
                }
            }
        }

        ni.left_neighbors = vec![Vec::new(); ni.node_count];
        ni.right_neighbors = vec![Vec::new(); ni.node_count];

        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id = node_id(&node);
                if node_id >= ni.node_count {
                    continue;
                }
                let mut right: Vec<Pair<LNodeRef, LEdgeRef>> = Vec::new();
                let mut max_priority = 0;
                let outgoing_edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let skip = edge
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.is_self_loop() || edge_guard.is_in_layer_edge())
                        .unwrap_or(false);
                    if skip {
                        continue;
                    }
                    let prio = edge
                        .lock()
                        .ok()
                        .and_then(|mut edge_guard| {
                            edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                        })
                        .unwrap_or(0);
                    if prio > max_priority {
                        max_priority = prio;
                        right.clear();
                    }
                    if prio == max_priority {
                        let target_node = edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.target())
                            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                        if let Some(target_node) = target_node {
                            right.push(Pair::of(target_node, edge.clone()));
                        }
                    }
                }
                right.sort_by(|a, b| neighbor_cmp(&ni, a, b));
                ni.right_neighbors[node_id] = right;

                let mut left: Vec<Pair<LNodeRef, LEdgeRef>> = Vec::new();
                let mut max_priority = 0;
                let incoming_edges = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.incoming_edges().clone())
                    .unwrap_or_default();
                for edge in incoming_edges {
                    let skip = edge
                        .lock()
                        .ok()
                        .map(|edge_guard| edge_guard.is_self_loop() || edge_guard.is_in_layer_edge())
                        .unwrap_or(false);
                    if skip {
                        continue;
                    }
                    let prio = edge
                        .lock()
                        .ok()
                        .and_then(|mut edge_guard| {
                            edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                        })
                        .unwrap_or(0);
                    if prio > max_priority {
                        max_priority = prio;
                        left.clear();
                    }
                    if prio == max_priority {
                        let source_node = edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.source())
                            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                        if let Some(source_node) = source_node {
                            left.push(Pair::of(source_node, edge.clone()));
                        }
                    }
                }
                left.sort_by(|a, b| neighbor_cmp(&ni, a, b));
                ni.left_neighbors[node_id] = left;
            }
        }

        ni
    }

    pub fn cleanup(&mut self) {
        self.layer_index.clear();
        self.node_index.clear();
        self.left_neighbors.clear();
        self.right_neighbors.clear();
        self.node_count = 0;
    }
}

fn neighbor_cmp(
    ni: &NeighborhoodInformation,
    a: &Pair<LNodeRef, LEdgeRef>,
    b: &Pair<LNodeRef, LEdgeRef>,
) -> Ordering {
    let a_id = node_id(&a.first);
    let b_id = node_id(&b.first);
    let a_index = ni.node_index.get(a_id).copied().unwrap_or(0);
    let b_index = ni.node_index.get(b_id).copied().unwrap_or(0);
    a_index.cmp(&b_index)
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}
