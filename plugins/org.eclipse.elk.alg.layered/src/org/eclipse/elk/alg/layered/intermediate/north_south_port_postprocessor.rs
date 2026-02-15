use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LGraphUtil, LNode, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;

pub struct NorthSouthPortPostprocessor;

impl Default for NorthSouthPortPostprocessor {
    fn default() -> Self {
        NorthSouthPortPostprocessor
    }
}

impl ILayoutProcessor<LGraph> for NorthSouthPortPostprocessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Odd port side processing", 1.0);

        let routing = graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Polyline);

        let layers = graph.layers().clone();
        for layer in layers {
            let node_array = layer
                .lock()
                .ok()
                .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
                .unwrap_or_default();

            for node in node_array {
                let (node_type, ports, dummy_pos) = {
                    let mut node_guard = match node.lock() {
                        Ok(guard) => guard,
                        Err(_) => continue,
                    };
                    let node_type = node_guard.node_type();
                    let ports = node_guard.ports().clone();
                    let dummy_pos = *node_guard.shape().position_ref();
                    (node_type, ports, dummy_pos)
                };

                if node_type != NodeType::NorthSouthPort {
                    continue;
                }

                eprintln!("DEBUG: Found NorthSouthPort dummy with {} ports, pos=({}, {})",
                          ports.len(), dummy_pos.x, dummy_pos.y);

                if routing == EdgeRouting::Splines {
                    // Fall back to non-spline processing for now.
                }

                let same_origin = ports_same_origin(&ports);

                for port in &ports {
                    let has_in = port
                        .lock()
                        .ok()
                        .map(|port_guard| !port_guard.incoming_edges().is_empty())
                        .unwrap_or(false);
                    let has_out = port
                        .lock()
                        .ok()
                        .map(|port_guard| !port_guard.outgoing_edges().is_empty())
                        .unwrap_or(false);

                    if has_in {
                        process_input_port(port, dummy_pos.y, same_origin);
                    }
                    if has_out {
                        process_output_port(port, dummy_pos.y, same_origin);
                    }
                }

                LNode::set_layer(&node, None);
            }
        }

        monitor.done();
    }
}

fn ports_same_origin(ports: &[LPortRef]) -> bool {
    if ports.len() < 2 {
        return false;
    }
    let mut origin_port: Option<crate::org::eclipse::elk::alg::layered::graph::LPortRef> = None;
    for port in ports {
        let origin = port
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
        let Some(Origin::LPort(origin_ref)) = origin else {
            return false;
        };
        match &origin_port {
            Some(existing) => {
                if !std::sync::Arc::ptr_eq(existing, &origin_ref) {
                    return false;
                }
            }
            None => origin_port = Some(origin_ref),
        }
    }
    true
}

fn process_input_port(port: &LPortRef, dummy_y: f64, add_junction: bool) {
    let origin_port = port
        .lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    let x = origin_port
        .lock()
        .ok()
        .and_then(|port_guard| port_guard.absolute_anchor())
        .unwrap_or_else(KVector::new)
        .x;

    let edges = port
        .lock()
        .ok()
        .map(|port_guard| LGraphUtil::to_edge_array(port_guard.incoming_edges()))
        .unwrap_or_default();

    for edge in edges {
        LEdge::set_target(&edge, Some(origin_port.clone()));
        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.bend_points().add_last_values(x, dummy_y);
            if add_junction {
                let mut junction_points = edge_guard
                    .get_property(LayeredOptions::JUNCTION_POINTS)
                    .unwrap_or_else(KVectorChain::new);
                junction_points.add_vector(KVector::with_values(x, dummy_y));
                edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
            }
        }
    }
}

fn process_output_port(port: &LPortRef, dummy_y: f64, add_junction: bool) {
    let origin_port = port
        .lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    let x = origin_port
        .lock()
        .ok()
        .and_then(|port_guard| port_guard.absolute_anchor())
        .unwrap_or_else(KVector::new)
        .x;

    let edges = port
        .lock()
        .ok()
        .map(|port_guard| LGraphUtil::to_edge_array(port_guard.outgoing_edges()))
        .unwrap_or_default();

    eprintln!("DEBUG: process_output_port: x={}, dummy_y={}, edges={}", x, dummy_y, edges.len());

    for edge in edges {
        LEdge::set_source(&edge, Some(origin_port.clone()));
        if let Ok(mut edge_guard) = edge.lock() {
            eprintln!("DEBUG: Adding bend point ({}, {}) to edge", x, dummy_y);
            edge_guard.bend_points().add_first_values(x, dummy_y);
            if add_junction {
                let mut junction_points = edge_guard
                    .get_property(LayeredOptions::JUNCTION_POINTS)
                    .unwrap_or_else(KVectorChain::new);
                junction_points.add_vector(KVector::with_values(x, dummy_y));
                edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
            }
        }
    }
}
