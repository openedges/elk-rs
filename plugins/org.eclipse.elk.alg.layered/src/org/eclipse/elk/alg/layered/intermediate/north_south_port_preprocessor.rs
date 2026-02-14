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

                let (node_type, port_constraints, north_ports, mut south_ports, graph_ref) = {
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

                // Java iterates south ports in reversed order (right-to-left).
                south_ports.reverse();

                let Some(graph_ref) = graph_ref else {
                    continue;
                };

                if let Ok(mut node_guard) = node.lock() {
                    node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
                }

                if north_ports.is_empty() && south_ports.is_empty() {
                    continue;
                }

                let mut north_dummy_nodes = Vec::new();
                let mut south_dummy_nodes = Vec::new();
                let mut barycenter_associates = Vec::new();

                create_dummy_nodes(
                    &graph_ref,
                    &north_ports,
                    &mut north_dummy_nodes,
                    Some(&mut south_dummy_nodes),
                    &mut barycenter_associates,
                );
                create_dummy_nodes(
                    &graph_ref,
                    &south_ports,
                    &mut south_dummy_nodes,
                    None,
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
    mut opposing_side_dummy_nodes: Option<&mut Vec<LNodeRef>>,
    barycenter_associates: &mut Vec<LNodeRef>,
) {
    let mut in_ports = Vec::with_capacity(ports.len());
    let mut out_ports = Vec::with_capacity(ports.len());
    let mut in_out_ports = Vec::with_capacity(ports.len());
    let mut same_side_self_loop_edges = Vec::new();
    let mut north_south_self_loop_edges = Vec::new();

    for port in ports {
        let port_side = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.side())
            .unwrap_or(PortSide::Undefined);
        let outgoing = port
            .lock()
            .ok()
            .map(|port_guard| LGraphUtil::to_edge_array(port_guard.outgoing_edges()))
            .unwrap_or_default();

        for edge in outgoing {
            let (source_port, target_port) = match edge.lock() {
                Ok(edge_guard) => (edge_guard.source(), edge_guard.target()),
                Err(_) => (None, None),
            };
            let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                continue;
            };
            let source_node = source_port.lock().ok().and_then(|port_guard| port_guard.node());
            let target_node = target_port.lock().ok().and_then(|port_guard| port_guard.node());
            let (Some(source_node), Some(target_node)) = (source_node, target_node) else {
                continue;
            };

            if !std::sync::Arc::ptr_eq(&source_node, &target_node) {
                continue;
            }

            let target_side = target_port
                .lock()
                .ok()
                .map(|port_guard| port_guard.side())
                .unwrap_or(PortSide::Undefined);
            if port_side == target_side {
                same_side_self_loop_edges.push(edge);
            } else if port_side == PortSide::North && target_side == PortSide::South {
                north_south_self_loop_edges.push(edge);
            }
        }
    }

    for edge in north_south_self_loop_edges {
        if let Some(opposing_side_dummy_nodes) = opposing_side_dummy_nodes.as_deref_mut() {
            create_north_south_self_loop_dummy_nodes(
                graph,
                &edge,
                dummy_nodes,
                opposing_side_dummy_nodes,
                PortSide::East,
            );
        }
    }

    for edge in same_side_self_loop_edges {
        create_same_side_self_loop_dummy_node(graph, &edge, dummy_nodes);
    }

    for port in ports {
        let (has_in, has_out) = match port.lock() {
            Ok(port_guard) => (
                !port_guard.incoming_edges().is_empty(),
                !port_guard.outgoing_edges().is_empty(),
            ),
            Err(_) => (false, false),
        };
        if has_in && has_out {
            in_out_ports.push(port.clone());
        } else if has_in {
            in_ports.push(port.clone());
        } else if has_out {
            out_ports.push(port.clone());
        }
    }

    for in_port in in_ports {
        let dummy = create_dummy_node(graph, Some(&in_port), None, dummy_nodes);
        barycenter_associates.push(dummy);
    }

    for out_port in out_ports {
        let dummy = create_dummy_node(graph, None, Some(&out_port), dummy_nodes);
        barycenter_associates.push(dummy);
    }

    for in_out_port in in_out_ports {
        let dummy = create_dummy_node(graph, Some(&in_out_port), Some(&in_out_port), dummy_nodes);
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

fn create_same_side_self_loop_dummy_node(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    self_loop: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
    dummy_nodes: &mut Vec<LNodeRef>,
) {
    let source_port = self_loop.lock().ok().and_then(|edge_guard| edge_guard.source());
    let target_port = self_loop.lock().ok().and_then(|edge_guard| edge_guard.target());
    let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
        return;
    };

    let dummy = LNode::new(graph);
    if let Ok(mut dummy_guard) = dummy.lock() {
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
        dummy_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
        dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LEdge(self_loop.clone())));
    }

    let dummy_input_port = LPort::new();
    if let Ok(mut dummy_input_port_guard) = dummy_input_port.lock() {
        dummy_input_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(target_port.clone())),
        );
        dummy_input_port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&dummy_input_port, Some(dummy.clone()));

    let dummy_output_port = LPort::new();
    if let Ok(mut dummy_output_port_guard) = dummy_output_port.lock() {
        dummy_output_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(source_port.clone())),
        );
        dummy_output_port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&dummy_output_port, Some(dummy.clone()));

    if let Ok(mut source_port_guard) = source_port.lock() {
        source_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
    }
    if let Ok(mut target_port_guard) = target_port.lock() {
        target_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
    }

    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(self_loop, None);
    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(self_loop, None);

    dummy_nodes.push(dummy);
}

fn create_north_south_self_loop_dummy_nodes(
    graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    self_loop: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
    north_dummy_nodes: &mut Vec<LNodeRef>,
    south_dummy_nodes: &mut Vec<LNodeRef>,
    side: PortSide,
) {
    let source_port = self_loop.lock().ok().and_then(|edge_guard| edge_guard.source());
    let target_port = self_loop.lock().ok().and_then(|edge_guard| edge_guard.target());
    let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
        return;
    };

    let north_dummy = LNode::new(graph);
    if let Ok(mut north_dummy_guard) = north_dummy.lock() {
        north_dummy_guard.set_node_type(NodeType::NorthSouthPort);
        north_dummy_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
        if let Some(source_node) = source_port.lock().ok().and_then(|port_guard| port_guard.node()) {
            north_dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(source_node)));
        }
    }

    let north_dummy_output_port = LPort::new();
    if let Ok(mut north_dummy_output_port_guard) = north_dummy_output_port.lock() {
        north_dummy_output_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(source_port.clone())),
        );
        north_dummy_output_port_guard.set_side(side);
    }
    LPort::set_node(&north_dummy_output_port, Some(north_dummy.clone()));
    if let Ok(mut source_port_guard) = source_port.lock() {
        source_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(north_dummy.clone()));
    }

    let south_dummy = LNode::new(graph);
    if let Ok(mut south_dummy_guard) = south_dummy.lock() {
        south_dummy_guard.set_node_type(NodeType::NorthSouthPort);
        south_dummy_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));
        if let Some(target_node) = target_port.lock().ok().and_then(|port_guard| port_guard.node()) {
            south_dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(target_node)));
        }
    }

    let south_dummy_input_port = LPort::new();
    if let Ok(mut south_dummy_input_port_guard) = south_dummy_input_port.lock() {
        south_dummy_input_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(target_port.clone())),
        );
        south_dummy_input_port_guard.set_side(side);
    }
    LPort::set_node(&south_dummy_input_port, Some(south_dummy.clone()));
    if let Ok(mut target_port_guard) = target_port.lock() {
        target_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(south_dummy.clone()));
    }

    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(
        self_loop,
        Some(north_dummy_output_port),
    );
    crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(
        self_loop,
        Some(south_dummy_input_port),
    );

    north_dummy_nodes.insert(0, north_dummy);
    south_dummy_nodes.push(south_dummy);
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
