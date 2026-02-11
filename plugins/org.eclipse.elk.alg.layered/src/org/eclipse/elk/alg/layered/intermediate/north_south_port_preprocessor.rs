use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphUtil, LNode, LNodeRef, LPort, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;

pub struct NorthSouthPortPreprocessor;

impl Default for NorthSouthPortPreprocessor {
    fn default() -> Self {
        NorthSouthPortPreprocessor
    }
}

impl ILayoutProcessor<LGraph> for NorthSouthPortPreprocessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Odd port side processing", 1.0);

        let layers = graph.layers().clone();
        for layer in layers {
            let node_array = layer
                .lock()
                .ok()
                .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
                .unwrap_or_default();

            let mut pointer: isize = -1;
            for node in node_array {
                pointer += 1;

                let (node_type, port_constraints, north_ports, south_ports, graph_ref) = {
                    let mut node_guard = match node.lock() {
                        Ok(guard) => guard,
                        Err(_) => continue,
                    };
                    let node_type = node_guard.node_type();
                    let port_constraints = node_guard
                        .get_property(LayeredOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined);
                    let north_ports = node_guard
                        .ports()
                        .iter()
                        .filter(|port| {
                            port.lock()
                                .ok()
                                .map(|port_guard| port_guard.side() == PortSide::North)
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let south_ports = node_guard
                        .ports()
                        .iter()
                        .filter(|port| {
                            port.lock()
                                .ok()
                                .map(|port_guard| port_guard.side() == PortSide::South)
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let graph_ref = node_guard.graph();
                    (node_type, port_constraints, north_ports, south_ports, graph_ref)
                };

                if node_type != NodeType::Normal || !port_constraints.is_side_fixed() {
                    continue;
                }

                if north_ports.is_empty() && south_ports.is_empty() {
                    continue;
                }

                let Some(graph_ref) = graph_ref else {
                    continue;
                };

                if let Ok(mut node_guard) = node.lock() {
                    node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
                }

                let mut north_dummy_nodes = Vec::new();
                let mut south_dummy_nodes = Vec::new();
                let mut barycenter_associates = Vec::new();

                create_dummy_nodes(
                    &graph_ref,
                    &north_ports,
                    &mut north_dummy_nodes,
                    &mut barycenter_associates,
                );
                create_dummy_nodes(
                    &graph_ref,
                    &south_ports,
                    &mut south_dummy_nodes,
                    &mut barycenter_associates,
                );

                let insert_point = pointer as usize;
                let successor = node.clone();
                for dummy in &north_dummy_nodes {
                    LNode::set_layer_at_index(dummy, insert_point, Some(layer.clone()));
                    pointer += 1;

                    if let Ok(mut dummy_guard) = dummy.lock() {
                        dummy_guard
                            .set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
                    }

                    if !origin_port_allows_switch(dummy) {
                        if let Ok(mut dummy_guard) = dummy.lock() {
                            let mut constraints = dummy_guard
                                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                                .unwrap_or_default();
                            constraints.push(successor.clone());
                            dummy_guard.set_property(
                                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                                Some(constraints),
                            );
                        }
                    }
                }

                let predecessor = node.clone();
                for dummy in &south_dummy_nodes {
                    LNode::set_layer_at_index(dummy, (pointer + 1) as usize, Some(layer.clone()));
                    pointer += 1;

                    if let Ok(mut dummy_guard) = dummy.lock() {
                        dummy_guard
                            .set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
                    }

                    if !origin_port_allows_switch(dummy) {
                        if let Ok(mut pred_guard) = predecessor.lock() {
                            let mut constraints = pred_guard
                                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                                .unwrap_or_default();
                            constraints.push(dummy.clone());
                            pred_guard.set_property(
                                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                                Some(constraints),
                            );
                        }
                    }
                }

                if !barycenter_associates.is_empty() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.set_property(
                            InternalProperties::BARYCENTER_ASSOCIATES,
                            Some(barycenter_associates),
                        );
                    }
                }
            }
        }

        monitor.done();
    }
}

fn create_dummy_nodes(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    ports: &[LPortRef],
    dummy_nodes: &mut Vec<LNodeRef>,
    barycenter_associates: &mut Vec<LNodeRef>,
) {
    for port in ports {
        let (has_in, has_out) = match port.lock() {
            Ok(port_guard) => (
                !port_guard.incoming_edges().is_empty(),
                !port_guard.outgoing_edges().is_empty(),
            ),
            Err(_) => (false, false),
        };
        if !has_in && !has_out {
            continue;
        }
        let dummy = create_dummy_node(
            graph,
            if has_in { Some(port) } else { None },
            if has_out { Some(port) } else { None },
            dummy_nodes,
        );
        barycenter_associates.push(dummy);
    }
}

fn create_dummy_node(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    in_port: Option<&LPortRef>,
    out_port: Option<&LPortRef>,
    dummy_nodes: &mut Vec<LNodeRef>,
) -> LNodeRef {
    let dummy = LNode::new(graph);
    if let Ok(mut dummy_guard) = dummy.lock() {
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
        dummy_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
    }

    if let Some(in_port) = in_port {
        let dummy_input_port = LPort::new();
        if let Ok(mut port_guard) = dummy_input_port.lock() {
            port_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LPort(in_port.clone())),
            );
            port_guard.set_side(PortSide::West);
        }
        LPort::set_node(&dummy_input_port, Some(dummy.clone()));

        let edges = in_port
            .lock()
            .ok()
            .map(|port_guard| LGraphUtil::to_edge_array(port_guard.incoming_edges()))
            .unwrap_or_default();
        for edge in edges {
            crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(
                &edge,
                Some(dummy_input_port.clone()),
            );
        }

        if let Ok(mut port_guard) = in_port.lock() {
            port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
            if let Some(node) = port_guard.node() {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(node)));
                }
            }
        }
    }

    if let Some(out_port) = out_port {
        let dummy_output_port = LPort::new();
        if let Ok(mut port_guard) = dummy_output_port.lock() {
            port_guard.set_property(
                InternalProperties::ORIGIN,
                Some(Origin::LPort(out_port.clone())),
            );
            port_guard.set_side(PortSide::East);
        }
        LPort::set_node(&dummy_output_port, Some(dummy.clone()));

        let edges = out_port
            .lock()
            .ok()
            .map(|port_guard| LGraphUtil::to_edge_array(port_guard.outgoing_edges()))
            .unwrap_or_default();
        for edge in edges {
            crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(
                &edge,
                Some(dummy_output_port.clone()),
            );
        }

        if let Ok(mut port_guard) = out_port.lock() {
            port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
            if let Some(node) = port_guard.node() {
                if let Ok(mut dummy_guard) = dummy.lock() {
                    dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(node)));
                }
            }
        }
    }

    dummy_nodes.push(dummy.clone());
    dummy
}

fn origin_port_allows_switch(dummy: &LNodeRef) -> bool {
    let origin_port = dummy
        .lock()
        .ok()
        .and_then(|dummy_guard| dummy_guard.ports().first().cloned())
        .and_then(|port| {
            port.lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN))
        });
    let Some(Origin::LPort(origin_port)) = origin_port else {
        return false;
    };

    let (allows_switch, port_constraints, origin_node) = {
        let Ok(mut port_guard) = origin_port.lock() else {
            return false;
        };
        (
            port_guard
                .get_property(LayeredOptions::ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES)
                .unwrap_or(false),
            port_guard.get_property(LayeredOptions::PORT_CONSTRAINTS),
            port_guard.node(),
        )
    };

    let port_constraints = port_constraints
        .or_else(|| {
            origin_node.and_then(|node| {
                node.lock()
                    .ok()
                    .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
            })
        })
        .unwrap_or(PortConstraints::Undefined);

    if port_constraints.is_pos_fixed() {
        return false;
    }

    allows_switch
}
