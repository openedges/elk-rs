use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphUtil, LNode, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

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
            let node_array = {
                let layer_guard = layer.lock();
                LGraphUtil::to_node_array(layer_guard.nodes())
            };

            for node in node_array {
                let (node_type, ports, dummy_pos) = {
                    let mut node_guard = node.lock();
                    let node_type = node_guard.node_type();
                    let ports = node_guard.ports().clone();
                    let dummy_pos = *node_guard.shape().position_ref();
                    (node_type, ports, dummy_pos)
                };

                if node_type != NodeType::NorthSouthPort {
                    continue;
                }

                if routing == EdgeRouting::Splines {
                    // Spline routing: reroute edges and set SPLINE_NS_PORT_Y_COORD,
                    // but do NOT add bend points (Java: processSplineInput/OutputPort)
                    for port in &ports {
                        let (has_in, has_out) = {
                            let port_guard = port.lock();
                            (
                                !port_guard.incoming_edges().is_empty(),
                                !port_guard.outgoing_edges().is_empty(),
                            )
                        };

                        if has_in {
                            process_spline_input_port(port, dummy_pos.y);
                        }
                        if has_out {
                            process_spline_output_port(port, dummy_pos.y);
                        }
                    }

                    LNode::set_layer(&node, None);
                    continue;
                }

                let same_origin = ports_same_origin(&ports);

                for port in &ports {
                    let (has_in, has_out) = {
                        let port_guard = port.lock();
                        (
                            !port_guard.incoming_edges().is_empty(),
                            !port_guard.outgoing_edges().is_empty(),
                        )
                    };

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
            .get_property(InternalProperties::ORIGIN);
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
        .get_property(InternalProperties::ORIGIN);
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    let x = origin_port
        .lock().absolute_anchor()
        .unwrap_or_else(KVector::new)
        .x;

    let edges = {
        let port_guard = port.lock();
        LGraphUtil::to_edge_array(port_guard.incoming_edges())
    };

    for edge in edges {
        LEdge::set_target(&edge, Some(origin_port.clone()));
        {
            let mut edge_guard = edge.lock();
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
        .get_property(InternalProperties::ORIGIN);
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    let x = origin_port
        .lock().absolute_anchor()
        .unwrap_or_else(KVector::new)
        .x;

    let edges = {
        let port_guard = port.lock();
        LGraphUtil::to_edge_array(port_guard.outgoing_edges())
    };

    for edge in edges {
        LEdge::set_source(&edge, Some(origin_port.clone()));
        {
            let mut edge_guard = edge.lock();
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

/// Spline-specific input port processing: reroute edges to origin port and set
/// SPLINE_NS_PORT_Y_COORD, but do NOT add bend points (Java: processSplineInputPort)
fn process_spline_input_port(port: &LPortRef, dummy_y: f64) {
    let origin_port = port
        .lock()
        .get_property(InternalProperties::ORIGIN);
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    // Set SPLINE_NS_PORT_Y_COORD on the origin port
    {
        let mut origin_guard = origin_port.lock();
        origin_guard.set_property(InternalProperties::SPLINE_NS_PORT_Y_COORD, Some(dummy_y));
    }

    // Reroute edges to origin port (no bend points added)
    let edges = {
        let port_guard = port.lock();
        LGraphUtil::to_edge_array(port_guard.incoming_edges())
    };

    for edge in edges {
        LEdge::set_target(&edge, Some(origin_port.clone()));
    }
}

/// Spline-specific output port processing: reroute edges to origin port and set
/// SPLINE_NS_PORT_Y_COORD, but do NOT add bend points (Java: processSplineOutputPort)
fn process_spline_output_port(port: &LPortRef, dummy_y: f64) {
    let origin_port = port
        .lock()
        .get_property(InternalProperties::ORIGIN);
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return;
    };

    // Set SPLINE_NS_PORT_Y_COORD on the origin port
    {
        let mut origin_guard = origin_port.lock();
        origin_guard.set_property(InternalProperties::SPLINE_NS_PORT_Y_COORD, Some(dummy_y));
    }

    // Reroute edges to origin port (no bend points added)
    let edges = {
        let port_guard = port.lock();
        LGraphUtil::to_edge_array(port_guard.outgoing_edges())
    };

    for edge in edges {
        LEdge::set_source(&edge, Some(origin_port.clone()));
    }
}
