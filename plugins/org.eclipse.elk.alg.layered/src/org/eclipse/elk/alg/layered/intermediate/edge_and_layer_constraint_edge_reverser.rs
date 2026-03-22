use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeConstraint, InternalProperties, LayerConstraint, LayeredOptions, PortType,
};

pub struct EdgeAndLayerConstraintEdgeReverser;

impl ILayoutProcessor<LGraph> for EdgeAndLayerConstraintEdgeReverser {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Edge and layer constraint edge reversal", 1.0);

        let remaining_nodes = handle_outer_nodes(layered_graph);
        handle_inner_nodes(layered_graph, &remaining_nodes);

        monitor.done();
    }
}

fn handle_outer_nodes(layered_graph: &mut LGraph) -> Vec<LNodeRef> {
    let mut remaining_nodes = Vec::with_capacity(layered_graph.layerless_nodes().len());
    for node in layered_graph.layerless_nodes().clone() {
        let layer_constraint = layer_constraint_of(&node);
        let edge_constraint = edge_constraint_for(layer_constraint);

        if let Some(edge_constraint) = edge_constraint {
            {
                // Java parity: EdgeAndLayerConstraintEdgeReverser stores OUTGOING_ONLY
                // regardless of whether the computed edge constraint is incoming or outgoing.
                let mut node_guard = node.lock();
                node_guard.set_property(
                    InternalProperties::EDGE_CONSTRAINT,
                    Some(EdgeConstraint::OutgoingOnly),
                );
            }
            reverse_edges(
                &node,
                layer_constraint,
                match edge_constraint {
                    EdgeConstraint::IncomingOnly => PortType::Input,
                    EdgeConstraint::OutgoingOnly => PortType::Output,
                    EdgeConstraint::None => PortType::Undefined,
                },
            );
        } else {
            remaining_nodes.push(node);
        }
    }
    remaining_nodes
}

fn handle_inner_nodes(_layered_graph: &mut LGraph, remaining_nodes: &[LNodeRef]) {
    for node in remaining_nodes {
        let layer_constraint = layer_constraint_of(node);
        let edge_constraint = edge_constraint_for(layer_constraint);

        if let Some(edge_constraint) = edge_constraint {
            {
                // Java parity: EdgeAndLayerConstraintEdgeReverser stores OUTGOING_ONLY
                // regardless of whether the computed edge constraint is incoming or outgoing.
                let mut node_guard = node.lock();
                node_guard.set_property(
                    InternalProperties::EDGE_CONSTRAINT,
                    Some(EdgeConstraint::OutgoingOnly),
                );
            }
            reverse_edges(
                node,
                layer_constraint,
                match edge_constraint {
                    EdgeConstraint::IncomingOnly => PortType::Input,
                    EdgeConstraint::OutgoingOnly => PortType::Output,
                    EdgeConstraint::None => PortType::Undefined,
                },
            );
            continue;
        }

        let ports = node.lock().ports().clone();
        if ports.is_empty() {
            continue;
        }

        let side_fixed = {
            let mut node_guard = node.lock();
            if node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(LayeredOptions::PORT_CONSTRAINTS)
            {
                node_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined)
            } else {
                PortConstraints::Undefined
            }
        }
        .is_side_fixed();
        if !side_fixed {
            continue;
        }

        let mut all_ports_reversed = true;
        for port in ports {
            let port_guard = port.lock();
            let side = port_guard.side();
            let net_flow = port_guard.net_flow();
            let outgoing_edges = port_guard.outgoing_edges().clone();
            let incoming_edges = port_guard.incoming_edges().clone();
            drop(port_guard);

            let looks_reversed = (side == PortSide::East && net_flow > 0)
                || (side == PortSide::West && net_flow < 0);
            if !looks_reversed {
                all_ports_reversed = false;
                break;
            }

            for edge in outgoing_edges {
                let target_node = {
                    let edge_guard = edge.lock();
                    edge_guard.target()
                };
                let target_constraint = if let Some(port) = target_node {
                    let port_guard = port.lock();
                    if let Some(node_ref) = port_guard.node() {
                        layer_constraint_of(&node_ref)
                    } else {
                        LayerConstraint::None
                    }
                } else {
                    LayerConstraint::None
                };
                if target_constraint == LayerConstraint::Last
                    || target_constraint == LayerConstraint::LastSeparate
                {
                    all_ports_reversed = false;
                    break;
                }
            }
            if !all_ports_reversed {
                break;
            }

            for edge in incoming_edges {
                let source_port = {
                    let edge_guard = edge.lock();
                    edge_guard.source()
                };
                let source_constraint = if let Some(port) = source_port {
                    let port_guard = port.lock();
                    if let Some(node_ref) = port_guard.node() {
                        layer_constraint_of(&node_ref)
                    } else {
                        LayerConstraint::None
                    }
                } else {
                    LayerConstraint::None
                };
                if source_constraint == LayerConstraint::First
                    || source_constraint == LayerConstraint::FirstSeparate
                {
                    all_ports_reversed = false;
                    break;
                }
            }
            if !all_ports_reversed {
                break;
            }
        }

        if all_ports_reversed {
            reverse_edges(node, layer_constraint, PortType::Undefined);
        }
    }
}

fn reverse_edges(
    node: &LNodeRef,
    node_layer_constraint: LayerConstraint,
    target_port_type: PortType,
) {
    let graph_ref = {
        let node_guard = node.lock();
        node_guard.graph().unwrap_or_default()
    };

    let ports = node.lock().ports().clone();

    for port in ports {
        if target_port_type == PortType::Input || target_port_type == PortType::Undefined {
            let outgoing = port.lock().outgoing_edges().clone();
            for edge in outgoing {
                if can_reverse_outgoing_edge(node_layer_constraint, &edge) {
                    reverse_edge(&graph_ref, &edge);
                }
            }
        }

        if target_port_type == PortType::Output || target_port_type == PortType::Undefined {
            let incoming = port.lock().incoming_edges().clone();
            for edge in incoming {
                if can_reverse_incoming_edge(node_layer_constraint, &edge) {
                    reverse_edge(&graph_ref, &edge);
                }
            }
        }
    }
}

fn reverse_edge(graph_ref: &LGraphRef, edge: &LEdgeRef) {
    LEdge::reverse(edge, graph_ref, true);
}

fn can_reverse_outgoing_edge(source_constraint: LayerConstraint, edge: &LEdgeRef) -> bool {
    {
        let mut edge_guard = edge.lock();
        if edge_guard
            .get_property(InternalProperties::REVERSED)
            .unwrap_or(false)
        {
            return false;
        }
    }

    let target_node = {
        let edge_guard = edge.lock();
        let target_port = edge_guard.target();
        target_port.and_then(|port| {
            let port_guard = port.lock();
            port_guard.node()
        })
    };
    let Some(target_node) = target_node else {
        return false;
    };

    if source_constraint == LayerConstraint::Last {
        let is_label = target_node.lock().node_type() == NodeType::Label;
        if is_label {
            return false;
        }
    }

    layer_constraint_of(&target_node) != LayerConstraint::LastSeparate
}

fn can_reverse_incoming_edge(target_constraint: LayerConstraint, edge: &LEdgeRef) -> bool {
    {
        let mut edge_guard = edge.lock();
        if edge_guard
            .get_property(InternalProperties::REVERSED)
            .unwrap_or(false)
        {
            return false;
        }
    }

    let source_node = {
        let edge_guard = edge.lock();
        let source_port = edge_guard.source();
        source_port.and_then(|port| {
            let port_guard = port.lock();
            port_guard.node()
        })
    };
    let Some(source_node) = source_node else {
        return false;
    };

    if target_constraint == LayerConstraint::First {
        let is_label = source_node.lock().node_type() == NodeType::Label;
        if is_label {
            return false;
        }
    }

    layer_constraint_of(&source_node) != LayerConstraint::FirstSeparate
}

fn layer_constraint_of(node: &LNodeRef) -> LayerConstraint {
    let mut node_guard = node.lock();
    if node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
    {
        node_guard
            .get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            .unwrap_or(LayerConstraint::None)
    } else {
        LayerConstraint::None
    }
}

fn edge_constraint_for(layer_constraint: LayerConstraint) -> Option<EdgeConstraint> {
    match layer_constraint {
        LayerConstraint::First | LayerConstraint::FirstSeparate => {
            Some(EdgeConstraint::OutgoingOnly)
        }
        LayerConstraint::Last | LayerConstraint::LastSeparate => Some(EdgeConstraint::IncomingOnly),
        LayerConstraint::None => None,
    }
}
