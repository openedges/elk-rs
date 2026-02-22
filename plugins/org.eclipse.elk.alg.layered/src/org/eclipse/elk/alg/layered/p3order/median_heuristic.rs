use std::any::Any;
use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;

pub struct MedianHeuristic;

impl MedianHeuristic {
    pub fn new() -> Self {
        MedianHeuristic
    }

    fn weight_of(node: &LNodeRef) -> Option<f64> {
        node.lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::WEIGHT))
    }

    fn weight_cmp(a: &LNodeRef, b: &LNodeRef) -> Ordering {
        let w1 = Self::weight_of(a);
        let w2 = Self::weight_of(b);
        if w1.is_none() && w2.is_none() {
            return Ordering::Greater;
        }
        if w1.is_none() {
            return Ordering::Greater;
        }
        if w2.is_none() {
            return Ordering::Less;
        }
        let w1 = w1.unwrap();
        let w2 = w2.unwrap();
        w1.partial_cmp(&w2).unwrap_or(Ordering::Equal)
    }

    fn stable_sort_by_weight(nodes: &mut Vec<LNodeRef>) {
        let mut entries: Vec<(usize, LNodeRef)> = nodes.iter().cloned().enumerate().collect();
        entries.sort_by(|(i, a), (j, b)| {
            let cmp = Self::weight_cmp(a, b);
            if cmp == Ordering::Equal {
                i.cmp(j)
            } else {
                cmp
            }
        });
        nodes.clear();
        nodes.extend(entries.into_iter().map(|(_, node)| node));
    }

    fn calculate_medians(&self, nodes: &[LNodeRef], reference_layer: isize) {
        let mut min_weight = f64::MIN_POSITIVE;
        let mut max_weight = f64::MAX;
        let mut to_revisit: Vec<LNodeRef> = Vec::new();

        for node in nodes {
            let mut connected_nodes: Vec<LNodeRef> = Vec::new();
            let edges: Vec<LEdgeRef> = node
                .lock()
                .ok()
                .map(|node_guard| {
                    node_guard
                        .incoming_edges()
                        .iter()
                        .chain(node_guard.outgoing_edges().iter())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            for edge in edges {
                let (source, target) = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| {
                        let source_node = edge_guard.source().and_then(|port| {
                            port.lock().ok().and_then(|port_guard| port_guard.node())
                        });
                        let target_node = edge_guard.target().and_then(|port| {
                            port.lock().ok().and_then(|port_guard| port_guard.node())
                        });
                        (source_node, target_node)
                    })
                    .unwrap_or((None, None));
                for candidate in [source, target].into_iter().flatten() {
                    if let Some(layer_id) = layer_id(&candidate) {
                        if layer_id as isize == reference_layer {
                            connected_nodes.push(candidate);
                        }
                    }
                }
            }

            if connected_nodes.is_empty() {
                to_revisit.push(node.clone());
            } else {
                Self::stable_sort_by_weight(&mut connected_nodes);
                let median = connected_nodes[connected_nodes.len() / 2].clone();
                if let Some(weight) = Self::weight_of(&median) {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.set_property(InternalProperties::WEIGHT, Some(weight));
                    }
                    min_weight = min_weight.min(weight);
                    max_weight = max_weight.max(weight);
                }
            }
        }

        let avg_weight = (max_weight + min_weight) / 2.0;
        for node in to_revisit {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::WEIGHT, Some(avg_weight));
            }
        }
    }
}

impl ICrossingMinimizationHeuristic for MedianHeuristic {
    fn always_improves(&self) -> bool {
        false
    }

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool, random: &mut Random) -> bool {
        let first_index = if forward_sweep {
            0
        } else {
            order.len().saturating_sub(1)
        };
        let Some(layer) = order.get(first_index) else {
            return false;
        };
        let mut first_layer = layer.to_vec();
        for node in &first_layer {
            let weight = random.next_double();
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::WEIGHT, Some(weight));
            }
        }
        Self::stable_sort_by_weight(&mut first_layer);
        for (index, node) in first_layer.into_iter().enumerate() {
            if let Some(layer_slot) = order.get_mut(first_index) {
                if index < layer_slot.len() {
                    layer_slot[index] = node.clone();
                }
            }
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::WEIGHT, Some((index + 1) as f64));
            }
        }
        false
    }

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        _is_first_sweep: bool,
        _random: &mut Random,
    ) -> bool {
        let reference_layer = if forward_sweep {
            free_layer_index as isize - 1
        } else {
            free_layer_index as isize + 1
        };
        if reference_layer < 0 || reference_layer as usize >= order.len() {
            return false;
        }
        let mut free_layer = order
            .get(free_layer_index)
            .map(|layer| layer.to_vec())
            .unwrap_or_default();
        self.calculate_medians(&free_layer, reference_layer);
        Self::stable_sort_by_weight(&mut free_layer);
        if let Some(layer_slot) = order.get_mut(free_layer_index) {
            for (index, node) in free_layer.into_iter().enumerate() {
                if index < layer_slot.len() {
                    layer_slot[index] = node;
                }
            }
        }
        false
    }

    fn is_deterministic(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl IInitializable for MedianHeuristic {}

fn layer_id(node: &LNodeRef) -> Option<usize> {
    node.lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
}
