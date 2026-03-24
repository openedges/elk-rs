use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct LongEdgeJoiner;

impl ILayoutProcessor<LGraph> for LongEdgeJoiner {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Edge joining", 1.0);

        let add_unnecessary_bendpoints = if layered_graph
            .graph_element()
            .properties()
            .has_property(LayeredOptions::UNNECESSARY_BENDPOINTS)
        {
            layered_graph
                .get_property(LayeredOptions::UNNECESSARY_BENDPOINTS)
                .unwrap_or(false)
        } else {
            false
        };
        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                let is_long_edge_dummy = node.lock().node_type() == NodeType::LongEdge;
                if !is_long_edge_dummy {
                    continue;
                }
                Self::join_at(&node, add_unnecessary_bendpoints);
                crate::org::eclipse::elk::alg::layered::graph::LNode::set_layer(&node, None);
            }
        }

        monitor.done();
    }
}

impl LongEdgeJoiner {
    pub fn join_at(long_edge_dummy: &LNodeRef, add_unnecessary_bendpoints: bool) {
        let west_port = long_edge_dummy
            .lock()
            .ports_by_side(PortSide::West)
            .first()
            .cloned();
        let east_port = long_edge_dummy
            .lock()
            .ports_by_side(PortSide::East)
            .first()
            .cloned();
        let (Some(west_port), Some(east_port)) = (west_port, east_port) else {
            return;
        };

        let mut input_edges = west_port
            .lock().incoming_edges().clone();
        let mut output_edges = east_port
            .lock().outgoing_edges().clone();
        let mut edge_count = input_edges.len().min(output_edges.len());

        let unnecessary_bendpoint = {
            let first_port = long_edge_dummy.lock().ports().first().cloned();
            first_port.and_then(|port| port.lock().absolute_anchor())
        };

        while edge_count > 0 {
            edge_count -= 1;
            let surviving_edge = input_edges.remove(0);
            let dropped_edge = output_edges.remove(0);

            let dropped_target = dropped_edge
                .lock().target();
            let Some(dropped_target) = dropped_target else {
                continue;
            };
            let dropped_edge_list_index = dropped_target
                .lock()
                .incoming_edges()
                .iter()
                .position(|candidate| Arc::ptr_eq(candidate, &dropped_edge))
                .unwrap_or(0);

            LEdge::set_target_and_insert_at_index(
                &surviving_edge,
                Some(dropped_target),
                dropped_edge_list_index,
            );
            LEdge::set_source(&dropped_edge, None);
            LEdge::set_target(&dropped_edge, None);

            if add_unnecessary_bendpoints {
                if let Some(unnecessary_bendpoint) = unnecessary_bendpoint {
                    {
                        let mut surviving_guard = surviving_edge.lock();
                        surviving_guard
                            .bend_points()
                            .add_vector(KVector::from_vector(&unnecessary_bendpoint));
                    }
                }
            }

            let dropped_bend_points = dropped_edge.lock().bend_points_ref().to_array();
            {
                let mut surviving_guard = surviving_edge.lock();
                surviving_guard.bend_points().add_all(&dropped_bend_points);
            }

            let dropped_labels = dropped_edge
                .lock().labels().clone();
            {
                let mut dropped_guard = dropped_edge.lock();
                dropped_guard.labels_mut().clear();
            }
            {
                let mut surviving_guard = surviving_edge.lock();
                surviving_guard.labels_mut().extend(dropped_labels);
            }

            let dropped_junction_points = {
                let mut edge_guard = dropped_edge.lock();
                if edge_guard
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::JUNCTION_POINTS)
                {
                    edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                } else {
                    None
                }
            };
            if let Some(dropped_junction_points) = dropped_junction_points {
                {
                    let mut surviving_guard = surviving_edge.lock();
                    let mut surviving_junctions = if surviving_guard
                        .graph_element()
                        .properties()
                        .has_property(LayeredOptions::JUNCTION_POINTS)
                    {
                        surviving_guard
                            .get_property(LayeredOptions::JUNCTION_POINTS)
                            .unwrap_or_else(KVectorChain::new)
                    } else {
                        KVectorChain::new()
                    };
                    surviving_junctions.add_all(&dropped_junction_points.to_array());
                    surviving_guard
                        .set_property(LayeredOptions::JUNCTION_POINTS, Some(surviving_junctions));
                }
            }
        }
    }
}
