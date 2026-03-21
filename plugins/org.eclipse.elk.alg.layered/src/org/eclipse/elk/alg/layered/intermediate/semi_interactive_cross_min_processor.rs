use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct SemiInteractiveCrossMinProcessor;

impl ILayoutProcessor<LGraph> for SemiInteractiveCrossMinProcessor {
    fn process(
        &mut self,
        layered_graph: &mut LGraph,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Semi-Interactive Crossing Minimization Processor", 1.0);

        let mut added_constraints = false;
        for layer in layered_graph.layers().clone() {
            let mut ordered_nodes: Vec<(f64, LNodeRef)> = Vec::new();
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let mut node_guard = match node.lock_ok() {
            Some(guard) => guard,
            None => continue,
                };
                if node_guard.node_type() != NodeType::Normal {
                    continue;
                }
                let has_position = node_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::POSITION);
                if !has_position {
                    continue;
                }
                if let Some(position) = node_guard.get_property(LayeredOptions::POSITION) {
                    ordered_nodes.push((position.y, node.clone()));
                }
            }

            ordered_nodes
                .sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(Ordering::Equal));

            for window in ordered_nodes.windows(2) {
                let prev = &window[0].1;
                let cur = &window[1].1;
                if let Some(mut prev_guard) = prev.lock_ok() {
                    let mut constraints = prev_guard
                        .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                        .unwrap_or_default();
                    constraints.push(cur.clone());
                    prev_guard.set_property(
                        InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                        Some(constraints),
                    );
                    added_constraints = true;
                }
            }
        }

        if added_constraints {
            layered_graph.set_property(
                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES,
                Some(true),
            );
        }

        progress_monitor.done();
    }
}
