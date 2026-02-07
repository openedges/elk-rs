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
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default()
            })
            .collect();

        for lnode in nodes {
            let has_self_loop_holder = lnode
                .lock()
                .ok()
                .is_some_and(|mut node_guard| {
                    node_guard.node_type() == NodeType::Normal
                        && node_guard
                            .get_property(InternalProperties::SELF_LOOP_HOLDER)
                            .is_some()
                });
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
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::SELF_LOOP_HOLDER));
    let Some(holder) = holder else {
        return;
    };

    let node_pos = node
        .lock()
        .ok()
        .map(|mut node_guard| *node_guard.shape().position_ref())
        .unwrap_or_default();

    let loops = holder
        .lock()
        .ok()
        .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
        .unwrap_or_default();

    for sl_loop in loops {
        let sl_edges = sl_loop
            .lock()
            .ok()
            .map(|loop_guard| loop_guard.sl_edges().clone())
            .unwrap_or_default();

        for sl_edge in sl_edges {
            let (l_edge, source_port, target_port) = sl_edge
                .lock()
                .ok()
                .map(|sl_edge_guard| {
                    (
                        sl_edge_guard.l_edge().clone(),
                        sl_edge_guard.sl_source().clone(),
                        sl_edge_guard.sl_target().clone(),
                    )
                })
                .unwrap_or_else(|| panic!("self loop edge lock poisoned"));

            let source_port = source_port
                .lock()
                .ok()
                .map(|port_guard| port_guard.l_port().clone())
                .unwrap_or_else(|| panic!("self loop source port lock poisoned"));
            let target_port = target_port
                .lock()
                .ok()
                .map(|port_guard| port_guard.l_port().clone())
                .unwrap_or_else(|| panic!("self loop target port lock poisoned"));

            LEdge::set_source(&l_edge, Some(source_port));
            LEdge::set_target(&l_edge, Some(target_port));

            if let Ok(mut edge_guard) = l_edge.lock() {
                edge_guard.bend_points().offset(node_pos.x, node_pos.y);
            };
        }
    }
}
