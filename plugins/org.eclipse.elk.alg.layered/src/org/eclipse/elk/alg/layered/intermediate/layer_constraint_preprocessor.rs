use rustc_hash::FxHashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredOptions,
};

pub struct LayerConstraintPreprocessor;

impl ILayoutProcessor<LGraph> for LayerConstraintPreprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layer constraint preprocessing", 1.0);

        let mut hidden_nodes = Vec::new();
        let mut hidden_connections = FxHashMap::<usize, HiddenNodeConnections>::default();

        let mut index = 0;
        while index < layered_graph.layerless_nodes().len() {
            let node = layered_graph.layerless_nodes()[index].clone();
            if is_relevant_node(&node) {
                hide(&node, &mut hidden_connections);
                hidden_nodes.push(node);
                layered_graph.layerless_nodes_mut().remove(index);
                continue;
            }
            index += 1;
        }

        if !hidden_nodes.is_empty() {
            layered_graph.set_property(InternalProperties::HIDDEN_NODES, Some(hidden_nodes));
        }

        monitor.done();
    }
}

fn is_relevant_node(node: &LNodeRef) -> bool {
    let constraint = layer_constraint_of(node);
    constraint == LayerConstraint::FirstSeparate || constraint == LayerConstraint::LastSeparate
}

fn hide(node: &LNodeRef, hidden_connections: &mut FxHashMap<usize, HiddenNodeConnections>) {
    ensure_no_inacceptable_edges(node);
    let connected_edges = node.lock().connected_edges();
    for edge in connected_edges {
        hide_edge(node, &edge, hidden_connections);
    }
}

fn hide_edge(
    hidden_node: &LNodeRef,
    edge: &LEdgeRef,
    hidden_connections: &mut FxHashMap<usize, HiddenNodeConnections>,
) {
    let is_outgoing = {
        let edge_guard = edge.lock();
        let source_port = edge_guard.source();
        if let Some(source) = source_port {
            let port_guard = source.lock();
            if let Some(source_node) = port_guard.node() {
                Arc::ptr_eq(&source_node, hidden_node)
            } else {
                false
            }
        } else {
            false
        }
    };

    let opposite_port = if is_outgoing {
        let edge_guard = edge.lock();
        edge_guard.target()
    } else {
        let edge_guard = edge.lock();
        edge_guard.source()
    };
    let Some(opposite_port) = opposite_port else {
        return;
    };

    if is_outgoing {
        crate::org::eclipse::elk::alg::layered::graph::LEdge::set_target(edge, None);
    } else {
        crate::org::eclipse::elk::alg::layered::graph::LEdge::set_source(edge, None);
    }
    {
        let mut edge_guard = edge.lock();
        edge_guard.set_property(
            InternalProperties::ORIGINAL_OPPOSITE_PORT,
            Some(opposite_port.clone()),
        );
    }

    let opposite_node = {
        let port_guard = opposite_port.lock();
        port_guard.node()
    };
    if let Some(opposite_node) = opposite_node {
        update_opposite_node_layer_constraints(hidden_node, &opposite_node, hidden_connections);
    }
}

fn update_opposite_node_layer_constraints(
    hidden_node: &LNodeRef,
    opposite_node: &LNodeRef,
    hidden_connections: &mut FxHashMap<usize, HiddenNodeConnections>,
) {
    let has_constraint = {
        let mut node_guard = opposite_node.lock();
        node_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
    };
    if has_constraint {
        return;
    }

    let hidden_constraint = layer_constraint_of(hidden_node);
    let key = node_ptr_key(opposite_node);
    let next_connection = hidden_connections
        .get(&key)
        .copied()
        .unwrap_or(HiddenNodeConnections::None)
        .combine(hidden_constraint);
    hidden_connections.insert(key, next_connection);

    let has_edges = {
        let node_guard = opposite_node.lock();
        !node_guard.connected_edges().is_empty()
    };
    if has_edges {
        return;
    }

    {
        let mut opposite_guard = opposite_node.lock();
        match next_connection {
            HiddenNodeConnections::FirstSeparate => {
                opposite_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::First),
                );
            }
            HiddenNodeConnections::LastSeparate => {
                opposite_guard.set_property(
                    LayeredOptions::LAYERING_LAYER_CONSTRAINT,
                    Some(LayerConstraint::Last),
                );
            }
            HiddenNodeConnections::None | HiddenNodeConnections::Both => {}
        }
    }
}

fn node_ptr_key(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}

fn ensure_no_inacceptable_edges(node: &LNodeRef) {
    let layer_constraint = layer_constraint_of(node);
    match layer_constraint {
        LayerConstraint::FirstSeparate => {
            let incoming = node.lock().incoming_edges();
            for edge in incoming {
                if !is_acceptable_incident_edge(&edge) {
                    let designation = node.lock().designation();
                    panic!(
                        "{}",
                        UnsupportedConfigurationException::new(format!(
                            "Node '{}' has its layer constraint set to FIRST_SEPARATE, but has at least one incoming edge. FIRST_SEPARATE nodes must not have incoming edges.",
                            designation
                        ))
                    );
                }
            }
        }
        LayerConstraint::LastSeparate => {
            let outgoing = node.lock().outgoing_edges();
            for edge in outgoing {
                if !is_acceptable_incident_edge(&edge) {
                    let designation = node.lock().designation();
                    panic!(
                        "{}",
                        UnsupportedConfigurationException::new(format!(
                            "Node '{}' has its layer constraint set to LAST_SEPARATE, but has at least one outgoing edge. LAST_SEPARATE nodes must not have outgoing edges.",
                            designation
                        ))
                    );
                }
            }
        }
        LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {}
    }
}

fn is_acceptable_incident_edge(edge: &LEdgeRef) -> bool {
    let source_type = {
        let edge_guard = edge.lock();
        let source_port = edge_guard.source();
        source_port.and_then(|source| {
            let port_guard = source.lock();
            let node = port_guard.node();
            node.map(|n| {
                let node_guard = n.lock();
                node_guard.node_type()
            })
        })
    };
    let target_type = {
        let edge_guard = edge.lock();
        let target_port = edge_guard.target();
        target_port.and_then(|target| {
            let port_guard = target.lock();
            let node = port_guard.node();
            node.map(|n| {
                let node_guard = n.lock();
                node_guard.node_type()
            })
        })
    };

    source_type == Some(NodeType::ExternalPort) && target_type == Some(NodeType::ExternalPort)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HiddenNodeConnections {
    None,
    FirstSeparate,
    LastSeparate,
    Both,
}

impl HiddenNodeConnections {
    fn combine(self, layer_constraint: LayerConstraint) -> Self {
        match self {
            HiddenNodeConnections::None => match layer_constraint {
                LayerConstraint::FirstSeparate => HiddenNodeConnections::FirstSeparate,
                LayerConstraint::LastSeparate => HiddenNodeConnections::LastSeparate,
                LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {
                    HiddenNodeConnections::None
                }
            },
            HiddenNodeConnections::FirstSeparate => match layer_constraint {
                LayerConstraint::FirstSeparate => HiddenNodeConnections::FirstSeparate,
                LayerConstraint::LastSeparate => HiddenNodeConnections::Both,
                LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {
                    HiddenNodeConnections::FirstSeparate
                }
            },
            HiddenNodeConnections::LastSeparate => match layer_constraint {
                LayerConstraint::FirstSeparate => HiddenNodeConnections::Both,
                LayerConstraint::LastSeparate => HiddenNodeConnections::LastSeparate,
                LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {
                    HiddenNodeConnections::LastSeparate
                }
            },
            HiddenNodeConnections::Both => HiddenNodeConnections::Both,
        }
    }
}
