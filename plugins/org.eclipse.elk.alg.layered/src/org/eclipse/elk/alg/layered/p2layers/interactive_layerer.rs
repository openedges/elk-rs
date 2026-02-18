use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LNode, LNodeRef, Layer};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::PortType;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

pub struct InteractiveLayerer;

impl InteractiveLayerer {
    pub fn new() -> Self {
        InteractiveLayerer
    }
}

impl Default for InteractiveLayerer {
    fn default() -> Self {
        Self::new()
    }
}

struct LayerSpan {
    start: f64,
    end: f64,
    nodes: Vec<LNodeRef>,
}

impl ILayoutPhase<LayeredPhases, LGraph> for InteractiveLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Interactive node layering", 1.0);

        let nodes = graph.layerless_nodes().clone();
        if nodes.is_empty() {
            monitor.done();
            return;
        }

        let graph_ref = nodes
            .first()
            .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()))
            .unwrap_or_default();

        let mut current_spans: Vec<LayerSpan> = Vec::new();
        for node in &nodes {
            let (minx, mut maxx) = node
                .lock()
                .ok()
                .map(|mut node_guard| {
                    let shape = node_guard.shape();
                    let pos = *shape.position_ref();
                    let size = *shape.size_ref();
                    (pos.x, pos.x + size.x)
                })
                .unwrap_or((0.0, 0.0));
            maxx = maxx.max(minx + 1.0);

            let mut idx = 0usize;
            let mut found_idx: Option<usize> = None;
            while idx < current_spans.len() {
                if current_spans[idx].start >= maxx {
                    break;
                }
                if current_spans[idx].end > minx {
                    if let Some(found) = found_idx {
                        let merged_nodes = current_spans[idx].nodes.clone();
                        let merged_end = current_spans[idx].end;
                        current_spans[found].nodes.extend(merged_nodes);
                        current_spans[found].end = current_spans[found].end.max(merged_end);
                        current_spans.remove(idx);
                        continue;
                    } else {
                        current_spans[idx].nodes.push(node.clone());
                        current_spans[idx].start = current_spans[idx].start.min(minx);
                        current_spans[idx].end = current_spans[idx].end.max(maxx);
                        found_idx = Some(idx);
                    }
                }
                idx += 1;
            }

            if found_idx.is_none() {
                let span = LayerSpan {
                    start: minx,
                    end: maxx,
                    nodes: vec![node.clone()],
                };
                current_spans.insert(idx, span);
            }
        }

        for (next_index, span) in current_spans.into_iter().enumerate() {
            let layer = Layer::new(&graph_ref);
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = next_index as i32;
            }
            graph.layers_mut().push(layer.clone());
            for node in span.nodes {
                LNode::set_layer(&node, Some(layer.clone()));
                set_node_id(&node, 0);
            }
        }

        for node in &nodes {
            if node_id(node) == 0 {
                let mut pending = VecDeque::from(check_node(node, graph, &graph_ref));
                let mut pending_set: HashSet<usize> = pending.iter().map(node_ptr_id).collect();
                while let Some(node_to_check) = pending.pop_front() {
                    let node_key = node_ptr_id(&node_to_check);
                    pending_set.remove(&node_key);
                    let new_nodes = check_node(&node_to_check, graph, &graph_ref);
                    for new_node in new_nodes {
                        let new_key = node_ptr_id(&new_node);
                        if !pending_set.contains(&new_key) {
                            pending.push_back(new_node.clone());
                            pending_set.insert(new_key);
                        }
                    }
                }
            }
        }

        graph.layers_mut().retain(|layer| {
            !layer
                .lock()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(false)
        });

        graph.layerless_nodes_mut().clear();
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .add_before(
                LayeredPhases::P1CycleBreaking,
                Arc::new(IntermediateProcessorStrategy::InteractiveExternalPortPositioner),
            )
            .add_before(
                LayeredPhases::P2Layering,
                Arc::new(IntermediateProcessorStrategy::LayerConstraintPreprocessor),
            )
            .add_before(
                LayeredPhases::P3NodeOrdering,
                Arc::new(IntermediateProcessorStrategy::LayerConstraintPostprocessor),
            );
        Some(config)
    }
}

fn check_node(node: &LNodeRef, graph: &mut LGraph, graph_ref: &LGraphRef) -> Vec<LNodeRef> {
    set_node_id(node, 1);
    let layer1 = node.lock().ok().and_then(|node_guard| node_guard.layer());
    let Some(layer1) = layer1 else {
        return Vec::new();
    };
    let layer1_id = layer1
        .lock()
        .ok()
        .map(|mut layer_guard| layer_guard.graph_element().id)
        .unwrap_or(0);

    let mut shifted_nodes: Vec<LNodeRef> = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    let ports = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports_by_type(PortType::Output))
        .unwrap_or_default();
    for port in ports {
        let outgoing = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.outgoing_edges().clone())
            .unwrap_or_default();
        for edge in outgoing {
            let target_node = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(target_node) = target_node else {
                continue;
            };
            if Arc::ptr_eq(node, &target_node) {
                continue;
            }
            let layer2 = target_node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.layer());
            let layer2_id = layer2
                .as_ref()
                .and_then(|layer| {
                    layer
                        .lock()
                        .ok()
                        .map(|mut layer_guard| layer_guard.graph_element().id)
                })
                .unwrap_or(-1);
            if layer2_id <= layer1_id {
                let new_index = (layer1_id + 1).max(0) as usize;
                if new_index == graph.layers().len() {
                    let new_layer = Layer::new(graph_ref);
                    if let Ok(mut layer_guard) = new_layer.lock() {
                        layer_guard.graph_element().id = layer1_id + 1;
                    }
                    graph.layers_mut().push(new_layer.clone());
                    LNode::set_layer(&target_node, Some(new_layer));
                } else if let Some(new_layer) = graph.layers().get(new_index).cloned() {
                    LNode::set_layer(&target_node, Some(new_layer));
                }

                let key = node_ptr_id(&target_node);
                if seen.insert(key) {
                    shifted_nodes.push(target_node.clone());
                }
            }
        }
    }

    shifted_nodes
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

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}
