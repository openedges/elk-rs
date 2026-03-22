use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct SelfLoopPostProcessor;

impl ILayoutProcessor<LGraph> for SelfLoopPostProcessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Self-Loop post-processing", 1.0);

        let nodes: Vec<LNodeRef> = graph
            .layers()
            .iter()
            .flat_map(|layer| {
                layer
                    .lock().nodes().clone()
            })
            .collect();

        for lnode in nodes {
            let has_self_loop_holder = {
                let mut node_guard = lnode.lock();
                node_guard.node_type() == NodeType::Normal
                    && node_guard
                        .get_property(InternalProperties::SELF_LOOP_HOLDER)
                        .is_some()
            };
            if !has_self_loop_holder {
                continue;
            }
            process_node(&lnode);
        }

        monitor.done();
    }
}

fn process_node(node: &LNodeRef) {
    let holder = node
        .lock()
        .get_property(InternalProperties::SELF_LOOP_HOLDER);
    let Some(holder) = holder else {
        return;
    };

    let node_pos = *node.lock().shape().position_ref();

    let loops = holder
        .lock().sl_hyper_loops().clone();

    for sl_loop in loops {
        let label_refs = {
            let mut loop_guard = sl_loop.lock();
            if let Some(labels) = loop_guard.sl_labels_mut() {
                let pos = labels.position_mut();
                pos.x += node_pos.x;
                pos.y += node_pos.y;
                Some(labels.l_labels().clone())
            } else {
                None
            }
        };

        if let Some(label_refs) = label_refs {
            for label in label_refs {
                {
                    let mut label_guard = label.lock();
                    label_guard.shape().position().x += node_pos.x;
                    label_guard.shape().position().y += node_pos.y;
                }
            }
        }

        let sl_edges = sl_loop
            .lock().sl_edges().clone();

        for sl_edge in sl_edges {
            let (l_edge, source_port, target_port) = {
                let sl_edge_guard = sl_edge.lock();
                (
                    sl_edge_guard.l_edge().clone(),
                    sl_edge_guard.sl_source().clone(),
                    sl_edge_guard.sl_target().clone(),
                )
            };

            let source_port = source_port.lock().l_port().clone();
            let target_port = target_port.lock().l_port().clone();

            LEdge::set_source(&l_edge, Some(source_port));
            LEdge::set_target(&l_edge, Some(target_port));

            {
                let mut edge_guard = l_edge.lock();
                edge_guard.bend_points().offset(node_pos.x, node_pos.y);
            };
        }
    }
}
