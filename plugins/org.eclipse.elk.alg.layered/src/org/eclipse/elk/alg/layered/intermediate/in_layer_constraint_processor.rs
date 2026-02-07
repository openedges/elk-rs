use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode};
use crate::org::eclipse::elk::alg::layered::options::{InLayerConstraint, InternalProperties};

pub struct InLayerConstraintProcessor;

impl ILayoutProcessor<LGraph> for InLayerConstraintProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layer constraint edge reversal", 1.0);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            let mut top_insertion_index: Option<usize> = None;
            let mut bottom_constrained_nodes = Vec::new();

            for (i, node) in nodes.iter().enumerate() {
                let constraint = node
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| {
                        if node_guard
                            .shape()
                            .graph_element()
                            .properties_mut()
                            .has_property(InternalProperties::IN_LAYER_CONSTRAINT)
                        {
                            node_guard.get_property(InternalProperties::IN_LAYER_CONSTRAINT)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(InLayerConstraint::None);

                if top_insertion_index.is_none() {
                    if constraint != InLayerConstraint::Top {
                        top_insertion_index = Some(i);
                    }
                } else if constraint == InLayerConstraint::Top {
                    let insert_index = top_insertion_index.unwrap_or(0);
                    LNode::set_layer(node, None);
                    LNode::set_layer_at_index(node, insert_index, Some(layer.clone()));
                    top_insertion_index = Some(insert_index + 1);
                }

                if constraint == InLayerConstraint::Bottom {
                    bottom_constrained_nodes.push(node.clone());
                }
            }

            for node in bottom_constrained_nodes {
                LNode::set_layer(&node, None);
                LNode::set_layer(&node, Some(layer.clone()));
            }
        }

        monitor.done();
    }
}
