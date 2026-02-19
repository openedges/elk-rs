use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::breaking_point_info::{
    BreakingPointInfo, BreakingPointInfoRef,
};
use crate::org::eclipse::elk::alg::layered::intermediate::single_edge_graph_wrapper::CuttingUtils;
use crate::org::eclipse::elk::alg::layered::intermediate::LongEdgeJoiner;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct BreakingPointProcessor;

impl ILayoutProcessor<LGraph> for BreakingPointProcessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Breaking Point Processor", 1.0);

        self.perform_wrapping(graph);

        if graph
            .get_property(LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES)
            .unwrap_or(true)
        {
            for layer in graph.layers().clone() {
                let nodes = layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default();
                for (index, node) in nodes.iter().enumerate() {
                    set_node_id(node, index as i32);
                }
            }

            self.improve_multi_cut_index_edges(graph);
            self.improve_unnecessarily_long_edges(graph, true);
            self.improve_unnecessarily_long_edges(graph, false);
        }

        progress_monitor.done();
    }
}

impl BreakingPointProcessor {
    fn perform_wrapping(&self, graph: &mut LGraph) {
        let graph_ref = graph_ref_for(graph);

        graph.layers_mut().insert(0, Layer::new(&graph_ref));

        let mut reverse = false;
        let mut idx = 1i32;
        let mut layer_it_index = 1usize;

        while layer_it_index < graph.layers().len() {
            if idx < 0 {
                break;
            }
            let new_layer_index = idx as usize;
            if new_layer_index >= graph.layers().len() {
                break;
            }

            let layer = graph.layers()[layer_it_index].clone();
            let new_layer = graph.layers()[new_layer_index].clone();

            let nodes_to_move = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            let offset = nodes_to_move.len();

            for node in &nodes_to_move {
                LNode::set_layer(node, Some(new_layer.clone()));
            }

            if reverse {
                for node in nodes_to_move.iter().rev() {
                    let incoming_edges = node
                        .lock()
                        .ok()
                        .map(|node_guard| node_guard.incoming_edges())
                        .unwrap_or_default();

                    for edge in incoming_edges {
                        LEdge::reverse(&edge, &graph_ref, true);
                        graph.set_property(InternalProperties::CYCLIC, Some(true));

                        let dummy_edges =
                            CuttingUtils::insert_dummies(graph, &graph_ref, &edge, offset);

                        let Some(bp_info) = breaking_point_info(node) else {
                            continue;
                        };
                        let Some(start_in_layer_edge) = dummy_edges.last().cloned() else {
                            continue;
                        };

                        let start_in_layer_dummy = start_in_layer_edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.source())
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            });
                        let end_in_layer_dummy = edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.target())
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            });

                        let lock_result = bp_info.lock();
                        if let Ok(mut bp_info_guard) = lock_result {
                            bp_info_guard.start_in_layer_dummy = start_in_layer_dummy;
                            bp_info_guard.start_in_layer_edge = Some(start_in_layer_edge);
                            bp_info_guard.end_in_layer_dummy = end_in_layer_dummy;
                            bp_info_guard.end_in_layer_edge = Some(edge);
                        };
                    }
                }

                reverse = false;
            } else if let Some(a_node) = nodes_to_move.first() {
                let is_breaking_point = a_node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type() == NodeType::BreakingPoint)
                    .unwrap_or(false);
                if is_breaking_point {
                    reverse = true;
                    idx = -1;
                }
            }

            idx += 1;
            layer_it_index += 1;
        }

        graph.layers_mut().retain(|layer| {
            !layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
        });
    }

    fn improve_multi_cut_index_edges(&self, graph: &mut LGraph) {
        for layer in graph.layers().clone() {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                if !is_start(&node) {
                    continue;
                }

                let Some(info) = breaking_point_info(&node) else {
                    continue;
                };

                let has_prev = info
                    .lock()
                    .ok()
                    .map(|info_guard| info_guard.prev.is_some())
                    .unwrap_or(false);
                let has_next = info
                    .lock()
                    .ok()
                    .map(|info_guard| info_guard.next.is_some())
                    .unwrap_or(false);
                if has_prev || !has_next {
                    continue;
                }

                let mut current = info;
                let mut next = current
                    .lock()
                    .ok()
                    .and_then(|info_guard| info_guard.next.clone());

                while let Some(next_info) = next {
                    let next_start = next_info
                        .lock()
                        .ok()
                        .map(|info_guard| info_guard.start.clone());
                    let next_start_in_layer_dummy = next_info
                        .lock()
                        .ok()
                        .and_then(|info_guard| info_guard.start_in_layer_dummy.clone());
                    let (Some(next_start), Some(next_start_in_layer_dummy)) =
                        (next_start, next_start_in_layer_dummy)
                    else {
                        break;
                    };

                    let _ = self.drop_dummies(&next_start, &next_start_in_layer_dummy, false, true);

                    let (
                        current_start,
                        current_end,
                        current_node_start_edge,
                        current_start_in_layer_dummy,
                        current_start_in_layer_edge,
                        current_end_in_layer_dummy,
                        current_end_in_layer_edge,
                        current_prev,
                    ) = if let Ok(current_guard) = current.lock() {
                        (
                            current_guard.start.clone(),
                            current_guard.end.clone(),
                            current_guard.node_start_edge.clone(),
                            current_guard.start_in_layer_dummy.clone(),
                            current_guard.start_in_layer_edge.clone(),
                            current_guard.end_in_layer_dummy.clone(),
                            current_guard.end_in_layer_edge.clone(),
                            current_guard.prev.clone(),
                        )
                    } else {
                        break;
                    };

                    let (
                        next_start,
                        next_end,
                        next_start_end_edge,
                        next_original_edge,
                        next_start_in_layer_dummy,
                        next_end_in_layer_dummy,
                        next_end_in_layer_edge,
                        next_next,
                    ) = if let Ok(next_guard) = next_info.lock() {
                        (
                            next_guard.start.clone(),
                            next_guard.end.clone(),
                            next_guard.start_end_edge.clone(),
                            next_guard.original_edge.clone(),
                            next_guard.start_in_layer_dummy.clone(),
                            next_guard.end_in_layer_dummy.clone(),
                            next_guard.end_in_layer_edge.clone(),
                            next_guard.next.clone(),
                        )
                    } else {
                        break;
                    };

                    let (
                        Some(current_start_in_layer_dummy),
                        Some(current_start_in_layer_edge),
                        Some(current_end_in_layer_edge),
                        Some(next_start_in_layer_dummy),
                        Some(next_end_in_layer_dummy),
                        Some(next_end_in_layer_edge),
                    ) = (
                        current_start_in_layer_dummy,
                        current_start_in_layer_edge,
                        current_end_in_layer_edge,
                        next_start_in_layer_dummy,
                        next_end_in_layer_dummy,
                        next_end_in_layer_edge,
                    )
                    else {
                        break;
                    };

                    update_indexes_after(&current_end);
                    update_indexes_after(&next_start);
                    update_indexes_after(&next_start_in_layer_dummy);
                    update_indexes_after(&next_end_in_layer_dummy);

                    let reconnect_target = current_end_in_layer_edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.target());
                    LEdge::set_target(&next_end_in_layer_edge, reconnect_target);
                    LEdge::set_target(&current_end_in_layer_edge, None);

                    LNode::set_layer(&current_end, None);
                    LNode::set_layer(&next_start, None);
                    LNode::set_layer(&next_start_in_layer_dummy, None);
                    LNode::set_layer(&next_end_in_layer_dummy, None);

                    let new_info = BreakingPointInfo::new(
                        current_start.clone(),
                        next_end.clone(),
                        current_node_start_edge,
                        next_start_end_edge,
                        next_original_edge,
                    );
                    if let Ok(mut new_info_guard) = new_info.lock() {
                        new_info_guard.start_in_layer_dummy = Some(current_start_in_layer_dummy);
                        new_info_guard.start_in_layer_edge = Some(current_start_in_layer_edge);
                        new_info_guard.end_in_layer_dummy = current_end_in_layer_dummy;
                        new_info_guard.end_in_layer_edge = Some(next_end_in_layer_edge);
                        new_info_guard.prev = current_prev;
                        new_info_guard.next = next_next;
                    }

                    if let Ok(mut start_guard) = current_start.lock() {
                        start_guard.set_property(
                            InternalProperties::BREAKING_POINT_INFO,
                            Some(new_info.clone()),
                        );
                    }
                    if let Ok(mut end_guard) = next_end.lock() {
                        end_guard.set_property(
                            InternalProperties::BREAKING_POINT_INFO,
                            Some(new_info.clone()),
                        );
                    }

                    let prev_ref = new_info
                        .lock()
                        .ok()
                        .and_then(|new_info_guard| new_info_guard.prev.clone());
                    if let Some(prev_ref) = prev_ref {
                        if let Ok(mut prev_guard) = prev_ref.lock() {
                            prev_guard.next = Some(new_info.clone());
                        }
                    }
                    let next_ref = new_info
                        .lock()
                        .ok()
                        .and_then(|new_info_guard| new_info_guard.next.clone());
                    if let Some(next_ref) = next_ref {
                        if let Ok(mut next_guard) = next_ref.lock() {
                            next_guard.prev = Some(new_info.clone());
                        }
                    }

                    next = new_info
                        .lock()
                        .ok()
                        .and_then(|new_info_guard| new_info_guard.next.clone());
                    current = new_info;
                }
            }
        }
    }

    fn improve_unnecessarily_long_edges(&self, graph: &mut LGraph, forwards: bool) {
        let check = if forwards { is_end } else { is_start };

        loop {
            let mut didsome = false;

            let mut layers = graph.layers().clone();
            if forwards {
                layers.reverse();
            }

            for layer in layers {
                let mut nodes = layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default();
                if !forwards {
                    nodes.reverse();
                }

                for node in nodes {
                    if !check(&node) {
                        continue;
                    }
                    let Some(bp_info) = breaking_point_info(&node) else {
                        continue;
                    };
                    let in_layer_dummy = if forwards {
                        bp_info
                            .lock()
                            .ok()
                            .and_then(|bp_info_guard| bp_info_guard.end_in_layer_dummy.clone())
                    } else {
                        bp_info
                            .lock()
                            .ok()
                            .and_then(|bp_info_guard| bp_info_guard.start_in_layer_dummy.clone())
                    };
                    if let Some(in_layer_dummy) = in_layer_dummy {
                        didsome = self.drop_dummies(&node, &in_layer_dummy, forwards, false);
                    }
                }
            }

            if !didsome {
                break;
            }
        }
    }

    fn drop_dummies(
        &self,
        bp_node: &LNodeRef,
        in_layer_dummy: &LNodeRef,
        forwards: bool,
        force: bool,
    ) -> bool {
        let mut pred_one = self.next_long_edge_dummy(bp_node, forwards);
        let mut pred_two = self.next_long_edge_dummy(in_layer_dummy, forwards);

        let mut didsome = false;
        while let (Some(pred_one_node), Some(pred_two_node)) = (pred_one, pred_two) {
            if force
                || self.is_adjacent_or_separated_by_breaking_points(
                    &pred_one_node,
                    &pred_two_node,
                    forwards,
                )
            {
                let next_one = self.next_long_edge_dummy(&pred_one_node, forwards);
                let next_two = self.next_long_edge_dummy(&pred_two_node, forwards);

                update_indexes_after(in_layer_dummy);
                update_indexes_after(bp_node);

                let new_layer = pred_one_node
                    .lock()
                    .ok()
                    .and_then(|node_guard| node_guard.layer());

                LongEdgeJoiner::join_at(&pred_one_node, false);
                LongEdgeJoiner::join_at(&pred_two_node, false);

                if let Some(new_layer) = new_layer {
                    if forwards {
                        let pred_two_id = node_id(&pred_two_node);
                        let pred_one_id = node_id(&pred_one_node);

                        set_layer_at_index_clamped(in_layer_dummy, pred_two_id, &new_layer);
                        set_node_id(in_layer_dummy, pred_two_id);

                        set_layer_at_index_clamped(bp_node, pred_one_id + 1, &new_layer);
                        set_node_id(bp_node, pred_one_id);
                    } else {
                        let pred_two_id = node_id(&pred_two_node);
                        let pred_one_id = node_id(&pred_one_node);

                        set_layer_at_index_clamped(bp_node, pred_one_id, &new_layer);
                        set_node_id(bp_node, pred_one_id);

                        set_layer_at_index_clamped(in_layer_dummy, pred_two_id + 1, &new_layer);
                        set_node_id(in_layer_dummy, pred_two_id);
                    }
                }

                LNode::set_layer(&pred_one_node, None);
                LNode::set_layer(&pred_two_node, None);

                pred_one = next_one;
                pred_two = next_two;
                didsome = true;
            } else {
                break;
            }
        }

        didsome
    }

    fn is_adjacent_or_separated_by_breaking_points(
        &self,
        dummy1: &LNodeRef,
        dummy2: &LNodeRef,
        forwards: bool,
    ) -> bool {
        let layer = dummy1.lock().ok().and_then(|node_guard| node_guard.layer());
        let Some(layer) = layer else {
            return false;
        };

        let start = if forwards { dummy2 } else { dummy1 };
        let end = if forwards { dummy1 } else { dummy2 };

        let start_id = node_id(start);
        let end_id = node_id(end);
        if end_id <= start_id {
            return true;
        }

        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();

        for i in (start_id + 1)..end_id {
            let Some(node) = nodes.get(i as usize) else {
                return false;
            };
            let is_breaking = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::BreakingPoint)
                .unwrap_or(false);
            if !(is_breaking || self.is_in_layer_dummy(node)) {
                return false;
            }
        }

        true
    }

    fn next_long_edge_dummy(&self, start: &LNodeRef, forwards: bool) -> Option<LNodeRef> {
        let edges = if forwards {
            start
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default()
        } else {
            start
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default()
        };

        let start_layer = start.lock().ok().and_then(|node_guard| node_guard.layer());

        for edge in edges {
            let other = edge
                .lock()
                .ok()
                .map(|edge_guard| edge_guard.other_node(start));
            let Some(other) = other else {
                continue;
            };

            let is_long_edge = other
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
                .unwrap_or(false);
            if !is_long_edge {
                continue;
            }

            let other_layer = other.lock().ok().and_then(|node_guard| node_guard.layer());
            if let (Some(start_layer), Some(other_layer)) = (&start_layer, &other_layer) {
                if !Arc::ptr_eq(start_layer, other_layer) {
                    return Some(other);
                }
            }
        }

        None
    }

    fn is_in_layer_dummy(&self, node: &LNodeRef) -> bool {
        let is_long_edge = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
            .unwrap_or(false);
        if !is_long_edge {
            return false;
        }

        let node_layer = node.lock().ok().and_then(|node_guard| node_guard.layer());

        let connected_edges = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.connected_edges())
            .unwrap_or_default();
        for edge in connected_edges {
            let is_self_loop = edge
                .lock()
                .ok()
                .map(|edge_guard| edge_guard.is_self_loop())
                .unwrap_or(false);
            if is_self_loop {
                continue;
            }
            let other = edge
                .lock()
                .ok()
                .map(|edge_guard| edge_guard.other_node(node));
            let Some(other) = other else {
                continue;
            };
            let other_layer = other.lock().ok().and_then(|node_guard| node_guard.layer());
            if let (Some(node_layer), Some(other_layer)) = (&node_layer, &other_layer) {
                if Arc::ptr_eq(node_layer, other_layer) {
                    return true;
                }
            }
        }

        false
    }
}

fn breaking_point_info(node: &LNodeRef) -> Option<BreakingPointInfoRef> {
    node.lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::BREAKING_POINT_INFO))
}

fn is_start(node: &LNodeRef) -> bool {
    let Some(bp_info) = breaking_point_info(node) else {
        return false;
    };
    bp_info
        .lock()
        .ok()
        .map(|bp_info_guard| Arc::ptr_eq(&bp_info_guard.start, node))
        .unwrap_or(false)
}

fn is_end(node: &LNodeRef) -> bool {
    let Some(bp_info) = breaking_point_info(node) else {
        return false;
    };
    bp_info
        .lock()
        .ok()
        .map(|bp_info_guard| Arc::ptr_eq(&bp_info_guard.end, node))
        .unwrap_or(false)
}

fn update_indexes_after(node: &LNodeRef) {
    let current_id = node_id(node);
    let layer = node.lock().ok().and_then(|node_guard| node_guard.layer());
    let Some(layer) = layer else {
        return;
    };

    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for candidate in nodes.into_iter().skip((current_id + 1).max(0) as usize) {
        let old_id = node_id(&candidate);
        set_node_id(&candidate, old_id - 1);
    }
}

fn node_id(node: &LNodeRef) -> i32 {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id)
        .unwrap_or(0)
}

fn set_node_id(node: &LNodeRef, value: i32) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.shape().graph_element().id = value;
    }
}

fn set_layer_at_index_clamped(node: &LNodeRef, index: i32, layer: &LayerRef) {
    let max_len = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().len())
        .unwrap_or(0);
    let clamped = index.clamp(0, max_len as i32) as usize;
    LNode::set_layer_at_index(node, clamped, Some(layer.clone()));
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock()
            .ok()
            .and_then(|layer_guard| layer_guard.graph())
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().ok().and_then(|node_guard| node_guard.graph()) {
            return graph_ref;
        }
    }
    LGraph::new()
}
