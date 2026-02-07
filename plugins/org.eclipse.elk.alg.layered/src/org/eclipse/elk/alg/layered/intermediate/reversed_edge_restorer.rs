use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct ReversedEdgeRestorer;

impl ILayoutProcessor<LGraph> for ReversedEdgeRestorer {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Restoring reversed edges", 1.0);

        let placeholder_graph = crate::org::eclipse::elk::alg::layered::graph::LGraph::new();
        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        let reversed = edge
                            .lock()
                            .ok()
                            .and_then(|mut edge_guard| {
                                edge_guard.get_property(InternalProperties::REVERSED)
                            })
                            .unwrap_or(false);
                        if reversed {
                            LEdge::reverse(&edge, &placeholder_graph, false);
                        }
                    }
                }
            }
        }

        monitor.done();
    }
}
