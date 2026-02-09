use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, NodeType as LNodeType};
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InLayerConstraint, LayerConstraint, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

/// Interactive layout processor that assigns reasonable positions to external port dummy nodes.
///
/// For interactive layout (using InteractiveCycleBreaker or InteractiveLayerer), dummy nodes
/// such as external port dummies need positions assigned up front. This processor positions
/// them appropriately - e.g., westward external ports are positioned left of all other nodes.
///
/// Based on Java's InteractiveExternalPortPositioner.
pub struct InteractiveExternalPortPositioner;

impl ILayoutProcessor<LGraph> for InteractiveExternalPortPositioner {
    fn process(&mut self, graph: &mut LGraph, _monitor: &mut dyn IElkProgressMonitor) {
        // If the graph does not contain any external ports, nothing to do
        if !graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .map(|gp| gp.contains(&GraphProperties::ExternalPorts))
            .unwrap_or(false)
        {
            return;
        }

        // Find the minimum and maximum x and y coordinates of normal nodes
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        let nodes = graph.layerless_nodes();
        for node_ref in nodes {
            if let Ok(mut node) = node_ref.lock() {
                if node.node_type() == LNodeType::Normal {
                    let pos = *node.shape().position();
                    let size = *node.shape().size();
                    let margins = node
                        .get_property(CoreOptions::MARGINS)
                        .map(|m| m.clone())
                        .unwrap_or_default();

                    min_x = min_x.min(pos.x - margins.left);
                    max_x = max_x.max(pos.x + size.x + margins.right);
                    min_y = min_y.min(pos.y - margins.top);
                    max_y = max_y.max(pos.y + size.y + margins.bottom);
                }
            }
        }

        // Arbitrary spacing value to separate external port dummies from other nodes
        const ARBITRARY_SPACING: f64 = 10.0;

        // Assign reasonable coordinates to external port dummies
        let nodes = graph.layerless_nodes().clone();
        for node_ref in &nodes {
            if let Ok(mut node) = node_ref.lock() {
                let node_type = node.node_type();
                if node_type != LNodeType::Normal {
                    match node_type {
                        LNodeType::ExternalPort => {
                            // Check layer constraint for WEST/EAST ports
                            if let Some(lc) = node.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT) {
                                match lc {
                                    LayerConstraint::FirstSeparate => {
                                        // WEST port - position left of all nodes
                                        node.shape().position().x = min_x - ARBITRARY_SPACING;

                                        // Find Y coordinate from connected target node
                                        if let Some(y) = Self::find_y_from_targets(&node_ref) {
                                            node.shape().position().y = y;
                                        }
                                    }
                                    LayerConstraint::LastSeparate => {
                                        // EAST port - position right of all nodes
                                        node.shape().position().x = max_x + ARBITRARY_SPACING;

                                        // Find Y coordinate from connected source node
                                        if let Some(y) = Self::find_y_from_sources(&node_ref) {
                                            node.shape().position().y = y;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // Check in-layer constraint for NORTH/SOUTH ports
                            if let Some(ilc) = node.get_property(InternalProperties::IN_LAYER_CONSTRAINT) {
                                match ilc {
                                    InLayerConstraint::Top => {
                                        // NORTH port
                                        if let Some(x) = Self::find_north_south_x(&node_ref) {
                                            node.shape().position().x = x + ARBITRARY_SPACING;
                                        }
                                        node.shape().position().y = min_y - ARBITRARY_SPACING;
                                    }
                                    InLayerConstraint::Bottom => {
                                        // SOUTH port
                                        if let Some(x) = Self::find_north_south_x(&node_ref) {
                                            node.shape().position().x = x + ARBITRARY_SPACING;
                                        }
                                        node.shape().position().y = max_y + ARBITRARY_SPACING;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl InteractiveExternalPortPositioner {
    /// Find Y coordinate for WEST ports by looking at target nodes
    fn find_y_from_targets(node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef) -> Option<f64> {
        if let Ok(mut node) = node_ref.lock() {
            for edge_ref in node.outgoing_edges() {
                if let Ok(edge) = edge_ref.lock() {
                    if let Some(target_port) = edge.target() {
                        if let Ok(port) = target_port.lock() {
                            if let Some(target_node_ref) = port.node() {
                                if let Ok(mut target_node) = target_node_ref.lock() {
                                    let pos = *target_node.shape().position();
                                    let size = *target_node.shape().size();
                                    return Some(pos.y + size.y / 2.0);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Find Y coordinate for EAST ports by looking at source nodes
    fn find_y_from_sources(node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef) -> Option<f64> {
        if let Ok(mut node) = node_ref.lock() {
            for edge_ref in node.incoming_edges() {
                if let Ok(edge) = edge_ref.lock() {
                    if let Some(source_port) = edge.source() {
                        if let Ok(port) = source_port.lock() {
                            if let Some(source_node_ref) = port.node() {
                                if let Ok(mut source_node) = source_node_ref.lock() {
                                    let pos = *source_node.shape().position();
                                    let size = *source_node.shape().size();
                                    return Some(pos.y + size.y / 2.0);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Find X coordinate for NORTH/SOUTH ports
    fn find_north_south_x(node_ref: &crate::org::eclipse::elk::alg::layered::graph::LNodeRef) -> Option<f64> {
        if let Ok(mut node) = node_ref.lock() {
            let ports = node.ports();
            if ports.is_empty() {
                return None;
            }

            // External port dummies should have exactly one port
            let port_ref = &ports[0];
            if let Ok(port) = port_ref.lock() {
                let has_outgoing = !port.outgoing_edges().is_empty();
                let has_incoming = !port.incoming_edges().is_empty();

                if has_outgoing {
                    // Find minimum position of target nodes
                    let mut min = f64::INFINITY;
                    for edge_ref in port.outgoing_edges() {
                        if let Ok(edge) = edge_ref.lock() {
                            if let Some(target_port) = edge.target() {
                                if let Ok(tp) = target_port.lock() {
                                    if let Some(target_node_ref) = tp.node() {
                                        if let Ok(mut target_node) = target_node_ref.lock() {
                                            let pos = *target_node.shape().position();
                                            let margins = target_node
                                                .get_property(CoreOptions::MARGINS)
                                                .map(|m| m.clone())
                                                .unwrap_or_default();
                                            min = min.min(pos.x - margins.left);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if min != f64::INFINITY {
                        return Some(min);
                    }
                }

                if has_incoming {
                    // Find maximum position of source nodes
                    let mut max = f64::NEG_INFINITY;
                    for edge_ref in port.incoming_edges() {
                        if let Ok(edge) = edge_ref.lock() {
                            if let Some(source_port) = edge.source() {
                                if let Ok(sp) = source_port.lock() {
                                    if let Some(source_node_ref) = sp.node() {
                                        if let Ok(mut source_node) = source_node_ref.lock() {
                                            let pos = *source_node.shape().position();
                                            let size = *source_node.shape().size();
                                            let margins = source_node
                                                .get_property(CoreOptions::MARGINS)
                                                .map(|m| m.clone())
                                                .unwrap_or_default();
                                            max = max.max(pos.x + size.x + margins.right);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if max != f64::NEG_INFINITY {
                        return Some(max);
                    }
                }
            }
        }
        None
    }
}
