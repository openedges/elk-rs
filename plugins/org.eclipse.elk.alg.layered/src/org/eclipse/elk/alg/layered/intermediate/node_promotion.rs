use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, LNodeRef, Layer, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, NodePromotionStrategy,
};

pub struct NodePromotion;

impl ILayoutProcessor<LGraph> for NodePromotion {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node promotion heuristic", 1.0);

        let strategy = graph
            .get_property(LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY)
            .unwrap_or_default();
        match strategy {
            NodePromotionStrategy::ModelOrderLeftToRight => {
                model_order_node_promotion(graph, true);
            }
            NodePromotionStrategy::ModelOrderRightToLeft => {
                model_order_node_promotion(graph, false);
            }
            _ => {}
        }

        monitor.done();
    }
}

struct LayerNodeMap {
    layers: BTreeMap<i32, Vec<LNodeRef>>,
    reverse: HashMap<usize, i32>,
}

impl LayerNodeMap {
    fn from_graph(graph: &LGraph, left_to_right: bool) -> Self {
        let mut layers = BTreeMap::new();
        let mut reverse = HashMap::new();

        for (layer_id, layer_ref) in graph.layers().iter().enumerate() {
            let mut nodes = layer_ref
                .lock().nodes().clone();
            nodes.sort_by(|left, right| model_order_compare(left, right, left_to_right));

            for node in &nodes {
                reverse.insert(node_key(node), layer_id as i32);
            }
            layers.insert(layer_id as i32, nodes);
        }

        Self { layers, reverse }
    }

    fn min_key(&self) -> Option<i32> {
        self.layers.keys().next().copied()
    }

    fn max_key(&self) -> Option<i32> {
        self.layers.keys().next_back().copied()
    }

    fn key_count(&self) -> usize {
        self.layers.len()
    }

    fn key_of(&self, node: &LNodeRef) -> Option<i32> {
        self.reverse.get(&node_key(node)).copied()
    }

    fn values(&self, key: i32) -> Vec<LNodeRef> {
        self.layers.get(&key).cloned().unwrap_or_default()
    }

    fn put(&mut self, key: i32, node: &LNodeRef) {
        let node_ptr = node_key(node);
        if let Some(old_key) = self.reverse.get(&node_ptr).copied() {
            if let Some(old_nodes) = self.layers.get_mut(&old_key) {
                old_nodes.retain(|candidate| !Arc::ptr_eq(candidate, node));
            }
        }

        self.layers.entry(key).or_default().push(node.clone());
        self.reverse.insert(node_ptr, key);
    }
}

fn model_order_compare(left: &LNodeRef, right: &LNodeRef, left_to_right: bool) -> Ordering {
    match (model_order(left), model_order(right)) {
        (Some(left_order), Some(right_order)) => {
            if left_to_right {
                right_order.cmp(&left_order)
            } else {
                left_order.cmp(&right_order)
            }
        }
        _ => Ordering::Equal,
    }
}

fn model_order_node_promotion(graph: &mut LGraph, left_to_right: bool) {
    let mut layer_map = LayerNodeMap::from_graph(graph, left_to_right);
    if layer_map.key_count() < 2 {
        return;
    }

    let mut something_changed;
    loop {
        something_changed = false;

        let min_key = layer_map.min_key().unwrap_or(0);
        let max_key = layer_map.max_key().unwrap_or(0);
        let mut current_layer_id = if left_to_right {
            max_key - 1
        } else {
            min_key + 1
        };

        while if left_to_right {
            current_layer_id >= min_key
        } else {
            current_layer_id <= max_key
        } {
            let mut node_index = 0usize;
            loop {
                let current_layer = layer_map.values(current_layer_id);
                if node_index >= current_layer.len() {
                    break;
                }

                let node = current_layer[node_index].clone();
                let Some(node_model_order) = model_order(&node) else {
                    node_index += 1;
                    continue;
                };

                if left_to_right && current_layer_id == max_key
                    || !left_to_right && current_layer_id == min_key
                {
                    node_index += 1;
                    continue;
                }

                let mut shall_be_promoted = true;
                for other_node in &current_layer {
                    if let Some(other_model_order) = model_order(other_node) {
                        let blocked = left_to_right && node_model_order < other_model_order
                            || !left_to_right && node_model_order > other_model_order;
                        if blocked {
                            shall_be_promoted = false;
                            break;
                        }
                    }
                }
                if !shall_be_promoted {
                    node_index += 1;
                    continue;
                }

                let next_layer_id = if left_to_right {
                    current_layer_id + 1
                } else {
                    current_layer_id - 1
                };
                let next_layer = layer_map.values(next_layer_id);

                let mut model_order_allows_promotion = false;
                let mut promote_through_dummy_layer = true;
                let mut contains_labels = false;

                for next_layer_node in &next_layer {
                    if let Some(next_model_order) = model_order(next_layer_node) {
                        if !Arc::ptr_eq(next_layer_node, &node) {
                            model_order_allows_promotion |= if left_to_right {
                                next_model_order < node_model_order
                            } else {
                                next_model_order > node_model_order
                            };
                            promote_through_dummy_layer = false;
                        }
                    } else if !model_order_allows_promotion
                        && promote_through_dummy_layer
                        && node_type(next_layer_node) == NodeType::Label
                    {
                        contains_labels = true;
                        let node_connected_to_next_layer = if left_to_right {
                            first_incoming_source_node(next_layer_node)
                        } else {
                            first_outgoing_target_node(next_layer_node)
                        };

                        if let Some(node_connected_to_next_layer) = node_connected_to_next_layer {
                            if Arc::ptr_eq(&node_connected_to_next_layer, &node) {
                                let connected_node = if left_to_right {
                                    first_outgoing_target_node(next_layer_node)
                                } else {
                                    first_incoming_source_node(next_layer_node)
                                };
                                if let Some(connected_node) = connected_node {
                                    let connected_layer =
                                        layer_map.key_of(&connected_node).unwrap_or(next_layer_id);
                                    let source_layer = layer_map
                                        .key_of(&node_connected_to_next_layer)
                                        .unwrap_or(current_layer_id);
                                    let layer_distance = if left_to_right {
                                        connected_layer - source_layer
                                    } else {
                                        source_layer - connected_layer
                                    };
                                    if layer_distance <= 2 {
                                        promote_through_dummy_layer = false;
                                    }
                                }
                            }
                        }
                    }
                }

                if contains_labels && promote_through_dummy_layer {
                    let connected_node = if left_to_right {
                        first_outgoing_target_node(&node)
                    } else {
                        first_incoming_source_node(&node)
                    };
                    if let Some(connected_node) = connected_node {
                        let connected_layer = layer_map
                            .key_of(&connected_node)
                            .unwrap_or(current_layer_id);
                        let node_layer = layer_map.key_of(&node).unwrap_or(current_layer_id);
                        let layer_distance = if left_to_right {
                            connected_layer - node_layer
                        } else {
                            node_layer - connected_layer
                        };
                        if layer_distance <= 2 && node_type(&connected_node) == NodeType::Normal {
                            promote_through_dummy_layer = false;
                        }
                    }
                }

                if model_order_allows_promotion || promote_through_dummy_layer {
                    let mut queue = VecDeque::new();
                    let mut queued = HashSet::new();
                    for promoted in
                        promote_node_by_model_order(&node, left_to_right, &mut layer_map)
                    {
                        let key = node_key(&promoted);
                        if queued.insert(key) {
                            queue.push_back(promoted);
                        }
                    }

                    while let Some(node_to_promote) = queue.pop_front() {
                        queued.remove(&node_key(&node_to_promote));
                        for promoted in promote_node_by_model_order(
                            &node_to_promote,
                            left_to_right,
                            &mut layer_map,
                        ) {
                            let key = node_key(&promoted);
                            if queued.insert(key) {
                                queue.push_back(promoted);
                            }
                        }
                    }

                    node_index = node_index.saturating_sub(1);
                    something_changed = true;
                } else {
                    node_index += 1;
                }
            }

            current_layer_id += if left_to_right { -1 } else { 1 };
        }

        if !something_changed {
            break;
        }
    }

    apply_model_order_layers(graph, &layer_map);
}

fn promote_node_by_model_order(
    node: &LNodeRef,
    left_to_right: bool,
    layer_map: &mut LayerNodeMap,
) -> Vec<LNodeRef> {
    let Some(old_layer) = layer_map.key_of(node) else {
        return Vec::new();
    };
    let new_layer = if left_to_right {
        old_layer + 1
    } else {
        old_layer - 1
    };
    layer_map.put(new_layer, node);

    let mut nodes_to_promote = Vec::new();
    let mut seen = HashSet::new();
    let connected_edges = if left_to_right {
        node.lock().outgoing_edges()
    } else {
        node.lock().incoming_edges()
    };

    for edge in connected_edges {
        let next_node = if left_to_right {
            edge.lock().target()
                .and_then(|port| port.lock().node())
        } else {
            edge.lock().source()
                .and_then(|port| port.lock().node())
        };

        let Some(next_node) = next_node else {
            continue;
        };
        if layer_map.key_of(&next_node) == layer_map.key_of(node) {
            let key = node_key(&next_node);
            if seen.insert(key) {
                nodes_to_promote.push(next_node);
            }
        }
    }

    nodes_to_promote
}

fn apply_model_order_layers(graph: &mut LGraph, layer_map: &LayerNodeMap) {
    let existing_layers = graph.layers().clone();
    let graph_ref = existing_layers.first().and_then(|layer| {
        layer
            .lock().graph()
    });

    let mut layer_refs_by_key: BTreeMap<i32, _> = BTreeMap::new();
    for (idx, layer_ref) in existing_layers.iter().enumerate() {
        layer_refs_by_key.insert(idx as i32, layer_ref.clone());
    }

    for key in layer_map.layers.keys() {
        if layer_refs_by_key.contains_key(key) {
            continue;
        }
        let Some(graph_ref) = &graph_ref else {
            continue;
        };
        layer_refs_by_key.insert(*key, Layer::new(graph_ref));
    }

    for layer_ref in layer_refs_by_key.values() {
        {
            let mut layer_guard = layer_ref.lock();
            layer_guard.nodes_mut().clear();
        }
    }

    let mut ordered_non_empty_layers = Vec::new();
    for (key, nodes) in &layer_map.layers {
        if nodes.is_empty() {
            continue;
        }
        let Some(target_layer) = layer_refs_by_key.get(key).cloned() else {
            continue;
        };
        for (idx, node) in nodes.iter().enumerate() {
            LNode::set_layer_at_index(node, idx, Some(target_layer.clone()));
        }
        ordered_non_empty_layers.push(target_layer);
    }

    graph.layers_mut().clear();
    graph.layers_mut().extend(ordered_non_empty_layers);
}

fn model_order(node: &LNodeRef) -> Option<i32> {
    node.lock()
        .get_property(InternalProperties::MODEL_ORDER)
}

fn node_type(node: &LNodeRef) -> NodeType {
    node.lock().node_type()
}

fn first_incoming_source_node(node: &LNodeRef) -> Option<LNodeRef> {
    node.lock()
        .incoming_edges()
        .first()
        .cloned()
        .and_then(|edge| edge.lock().source())
        .and_then(|port| port.lock().node())
}

fn first_outgoing_target_node(node: &LNodeRef) -> Option<LNodeRef> {
    node.lock()
        .outgoing_edges()
        .first()
        .cloned()
        .and_then(|edge| edge.lock().target())
        .and_then(|port| port.lock().node())
}

fn node_key(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}
