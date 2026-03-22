use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct PartitionPostprocessor;

impl ILayoutProcessor<LGraph> for PartitionPostprocessor {
    fn process(&mut self, lgraph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Partition postprocessing", 1.0);

        let layers = lgraph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                let partition_ports: Vec<LPortRef> = {
                    let node_guard = node.lock();
                    node_guard
                        .ports()
                        .iter()
                        .filter_map(|port| {
                            let is_partition_dummy = port
                                .lock()
                                .get_property(InternalProperties::PARTITION_DUMMY)
                                .unwrap_or(false);
                            if is_partition_dummy {
                                Some(port.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                for port in partition_ports {
                    detach_partition_port(&port);
                }
            }
        }

        monitor.done();
    }
}

fn detach_partition_port(port: &LPortRef) {
    let connected_edges = port
        .lock().connected_edges();

    for edge in connected_edges {
        let source_is_port = edge
            .lock().source()
            .map(|source| Arc::ptr_eq(&source, port))
            .unwrap_or(false);
        if source_is_port {
            LEdge::set_source(&edge, None);
        }

        let target_is_port = edge
            .lock().target()
            .map(|target| Arc::ptr_eq(&target, port))
            .unwrap_or(false);
        if target_is_port {
            LEdge::set_target(&edge, None);
        }
    }

    crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(port, None);
}
