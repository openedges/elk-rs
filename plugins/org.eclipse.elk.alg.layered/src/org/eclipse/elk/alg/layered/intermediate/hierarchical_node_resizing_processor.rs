use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphUtil, LNode, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_label_placement::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};
use std::collections::HashSet;

fn trace_resize(message: &str) {
    if std::env::var("ELK_TRACE").is_ok() {
        eprintln!("[hierarchical-resize] {message}");
    }
}

/// This processor resizes a child graph to fit the parent node. It must be run as the last non-hierarchical processor in
/// a hierarchical graph.
///
/// # Precondition
/// graph with layout completed
///
/// # Postcondition
/// Graph is resized to fit parent node
///
/// # Slots
/// After phase 5.
pub struct HierarchicalNodeResizingProcessor;

impl Default for HierarchicalNodeResizingProcessor {
    fn default() -> Self {
        Self
    }
}

impl ILayoutProcessor<LGraph> for HierarchicalNodeResizingProcessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        trace_resize("start");
        progress_monitor.begin("Resize child graph to fit parent.", 1.0);

        // Move all nodes from all layers to layerless
        let layers = graph.layers().clone();
        for layer_ref in layers {
            if let Ok(mut layer_guard) = layer_ref.lock() {
                let layer_nodes = layer_guard.nodes().clone();
                graph.layerless_nodes_mut().extend(layer_nodes);
                layer_guard.nodes_mut().clear();
            }
        }
        trace_resize("moved nodes to layerless");

        // Clear layer assignments for all layerless nodes
        for node in graph.layerless_nodes().clone() {
            LNode::set_layer(&node, None);
        }

        // Clear all layers
        graph.layers_mut().clear();
        trace_resize("cleared layers");

        // Resize the graph
        resize_graph(graph);
        trace_resize("resized graph");

        // If nested, transfer layout to parent node
        if is_nested(graph) {
            if let Some(parent_node) = graph.parent_node() {
                trace_resize("transfer layout to parent");
                graph_layout_to_node(&parent_node, graph);
                trace_resize("transfer layout done");
            }
        }

        progress_monitor.done();
        trace_resize("done");
    }
}

/// Transfer the layout of the given graph to the given associated node.
///
/// # Arguments
/// * `node` - a compound node
/// * `lgraph` - the graph nested in the compound node
fn graph_layout_to_node(node: &LNodeRef, lgraph: &mut LGraph) {
    let trace_external_ports = std::env::var("ELK_TRACE_EXTERNAL_PORTS").is_ok();
    let (graph_padding, graph_offset) = if trace_external_ports {
        (lgraph.padding_ref().clone(), *lgraph.offset_ref())
    } else {
        (Default::default(), KVector::new())
    };
    // Process external ports
    let layerless_nodes = lgraph.layerless_nodes().clone();
    let east_edge_less_external_dummy_count = layerless_nodes
        .iter()
        .filter(|child_node| {
            let ext_port_side = child_node
                .lock()
                .ok()
                .and_then(|mut child_guard| child_guard.get_property(InternalProperties::EXT_PORT_SIDE))
                .unwrap_or(PortSide::Undefined);
            ext_port_side == PortSide::East && external_dummy_edge_count(child_node) == 0
        })
        .count();

    for child_node in &layerless_nodes {
        let (port_ref, ext_port_side, dummy_pos_y) = {
            let mut child_guard = match child_node.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            let port_ref = match child_guard.get_property(InternalProperties::ORIGIN) {
                Some(Origin::LPort(port_ref)) => port_ref,
                _ => continue,
            };
            let ext_port_side = child_guard
                .get_property(InternalProperties::EXT_PORT_SIDE)
                .unwrap_or(PortSide::Undefined);
            let dummy_pos_y = child_guard.shape().position_ref().y;
            (port_ref, ext_port_side, dummy_pos_y)
        };

        trace_resize("external port dummy found");

        // Get port size and external port side
        let port_size = if let Ok(mut port_guard) = port_ref.lock() {
            *port_guard.shape().size_ref()
        } else {
            continue;
        };

        let mut port_position =
            get_external_port_position_for_graph(lgraph, child_node, port_size.x, port_size.y);
        let allow_edge_less_self_loop =
            east_edge_less_external_dummy_count == 1 && external_dummy_edge_count(child_node) == 0;
        let pure_self_loop =
            ext_port_side == PortSide::East
                && is_pure_self_loop_external_dummy(child_node, allow_edge_less_self_loop);
        // For pure self-loop sources on the east side, Java effectively keeps the dummy's y-origin
        // at the content top. Compensate the dummy y-shift here before writing back to the parent port.
        if pure_self_loop {
            port_position.y -= dummy_pos_y;
        }
        if trace_external_ports {
            let (
                dummy_pos,
                dummy_size,
                dummy_id,
                dummy_layer_id,
                dummy_margin_top,
                dummy_margin_bottom,
            ) = child_node
                .lock()
                .ok()
                .map(|mut dummy_guard| {
                    (
                        *dummy_guard.shape().position_ref(),
                        *dummy_guard.shape().size_ref(),
                        dummy_guard.shape().graph_element().id,
                        dummy_guard
                            .get_property(LayeredOptions::LAYERING_LAYER_ID)
                            .unwrap_or(-1),
                        dummy_guard.margin().top,
                        dummy_guard.margin().bottom,
                    )
                })
                .unwrap_or((KVector::new(), KVector::new(), 0, -1, 0.0, 0.0));
            let port_id = port_ref
                .lock()
                .ok()
                .map(|mut port_guard| port_guard.shape().graph_element().id)
                .unwrap_or(-1);
            let label_debug = child_node
                .lock()
                .ok()
                .and_then(|dummy_guard| dummy_guard.ports().first().cloned())
                .and_then(|port| {
                    port.lock().ok().map(|port_guard| {
                        port_guard
                            .labels()
                            .iter()
                            .filter_map(|label| {
                                label.lock().ok().map(|mut label_guard| {
                                    let pos = *label_guard.shape().position_ref();
                                    let size = *label_guard.shape().size_ref();
                                    format!(
                                        "({:.1},{:.1})[{:.1}x{:.1}]",
                                        pos.x, pos.y, size.x, size.y
                                    )
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default()
                .join(",");
            eprintln!(
                "[ext-port] port_id={} dummy_id={} layer_id={} side={:?} pure_self_loop={} dummy_pos=({:.1},{:.1}) dummy_size=({:.1},{:.1}) margin=({:.1},{:.1}) labels=[{}] graph_pad=({:.1},{:.1},{:.1},{:.1}) graph_offset=({:.1},{:.1}) port_size=({:.1},{:.1}) port_pos=({:.1},{:.1})",
                port_id,
                dummy_id,
                dummy_layer_id,
                ext_port_side,
                pure_self_loop,
                dummy_pos.x,
                dummy_pos.y,
                dummy_size.x,
                dummy_size.y,
                dummy_margin_top,
                dummy_margin_bottom,
                label_debug,
                graph_padding.left,
                graph_padding.top,
                graph_padding.right,
                graph_padding.bottom,
                graph_offset.x,
                graph_offset.y,
                port_size.x,
                port_size.y,
                port_position.x,
                port_position.y
            );
        }

        // Set port position and side
        if let Ok(mut port_guard) = port_ref.lock() {
            let pos = port_guard.shape().position();
            pos.x = port_position.x;
            pos.y = port_position.y;
            port_guard.set_side(ext_port_side);
        };
    }

    // Setup the parent node
    trace_resize("setup parent node");
    let actual_graph_size = lgraph.actual_size();
    if std::env::var("ELK_TRACE_HN").is_ok() {
        if let Ok(mut node_guard) = node.lock() {
            let size_constraints = node_guard
                .get_property(LayeredOptions::NODE_SIZE_CONSTRAINTS)
                .unwrap_or_else(EnumSet::none_of);
            let min_size = node_guard
                .get_property(LayeredOptions::NODE_SIZE_MINIMUM)
                .unwrap_or_default();
            let size_opts = node_guard
                .shape()
                .get_property(CoreOptions::NODE_SIZE_OPTIONS)
                .unwrap_or_else(EnumSet::none_of);
            let lgraph_size = *lgraph.size_ref();
            let padding = lgraph.padding_ref().clone();
            eprintln!(
                "[hnr] node={:?} actual_graph_size={:?} lgraph_size={:?} padding=({},{},{},{}) min_size=({},{}) constraints={:?} size_opts={:?}",
                node_guard.shape().graph_element().id,
                actual_graph_size,
                lgraph_size,
                padding.left,
                padding.top,
                padding.bottom,
                padding.right,
                min_size.x,
                min_size.y,
                size_constraints,
                size_opts
            );
        }
    }
    let has_external_ports = lgraph
        .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
        .unwrap_or_else(EnumSet::none_of)
        .contains(&GraphProperties::ExternalPorts);

    if let Ok(mut node_guard) = node.lock() {
        if has_external_ports {
            // Ports have positions assigned
            node_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedPos),
            );

            // Add NON_FREE_PORTS to the parent graph's properties
            if let Some(parent_graph_ref) = node_guard.graph() {
                if let Ok(mut parent_graph_guard) = parent_graph_ref.lock() {
                    let mut graph_props = parent_graph_guard
                        .get_property(InternalProperties::GRAPH_PROPERTIES)
                        .unwrap_or_else(EnumSet::none_of);
                    graph_props.insert(GraphProperties::NonFreePorts);
                    parent_graph_guard
                        .set_property(InternalProperties::GRAPH_PROPERTIES, Some(graph_props));
                }
            }

            drop(node_guard); // Release before calling resize_node
            LGraphUtil::resize_node(node, &actual_graph_size, false, true);
        } else {
            // Ports have not been positioned yet - leave this for next layouter
            drop(node_guard); // Release before calling resize_node
            LGraphUtil::resize_node(node, &actual_graph_size, true, true);
        }
    }
}

fn is_nested(graph: &LGraph) -> bool {
    graph.parent_node().is_some()
}

fn is_pure_self_loop_external_dummy(node: &LNodeRef, allow_edge_less: bool) -> bool {
    let all_edges = external_dummy_edges(node);

    // For some hierarchical self-loop sources, the edge chain is already consumed before this
    // transfer step and the external dummy has no remaining incident edge.
    let all_self = if all_edges.is_empty() {
        allow_edge_less
    } else {
        all_edges.iter().all(edge_represents_self_loop)
    };
    all_self
}

fn external_dummy_edges(node: &LNodeRef) -> Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> {
    let ports = match node.lock() {
        Ok(node_guard) => node_guard.ports().clone(),
        Err(_) => return Vec::new(),
    };

    let mut all_edges = Vec::new();
    let mut seen = HashSet::new();
    for port in ports {
        let (incoming, outgoing) = match port.lock() {
            Ok(port_guard) => (
                port_guard.incoming_edges().clone(),
                port_guard.outgoing_edges().clone(),
            ),
            Err(_) => continue,
        };
        for edge in incoming.into_iter().chain(outgoing) {
            let edge_key = std::sync::Arc::as_ptr(&edge) as usize;
            if seen.insert(edge_key) {
                all_edges.push(edge);
            }
        }
    }

    all_edges
}

fn external_dummy_edge_count(node: &LNodeRef) -> usize {
    external_dummy_edges(node).len()
}

fn edge_represents_self_loop(
    edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
) -> bool {
    let origin_edge = {
        let mut edge_guard = match edge.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        if edge_guard.is_self_loop() {
            return true;
        }
        edge_guard
            .get_property(InternalProperties::ORIGIN)
            .and_then(|origin| match origin {
                Origin::LEdge(origin_edge) => Some(origin_edge),
                _ => None,
            })
    };

    origin_edge
        .and_then(|origin_edge| {
            origin_edge
                .lock()
                .ok()
                .map(|origin_edge_guard| origin_edge_guard.is_self_loop())
        })
        .unwrap_or(false)
}

fn get_external_port_position_for_graph(
    graph: &LGraph,
    port_dummy: &LNodeRef,
    port_width: f64,
    port_height: f64,
) -> KVector {
    let mut dummy_guard = match port_dummy.lock() {
        Ok(guard) => guard,
        Err(_) => return KVector::new(),
    };

    let pos = *dummy_guard.shape().position_ref();
    let size = *dummy_guard.shape().size_ref();
    let mut port_pos = KVector::from_vector(&pos);
    port_pos.x += size.x / 2.0;
    port_pos.y += size.y / 2.0;
    let port_offset = dummy_guard
        .get_property(LayeredOptions::PORT_BORDER_OFFSET)
        .unwrap_or(0.0);
    let ext_side = dummy_guard
        .get_property(InternalProperties::EXT_PORT_SIDE)
        .unwrap_or(PortSide::Undefined);

    let graph_size = *graph.size_ref();
    let padding = graph.padding_ref().clone();
    let graph_offset = *graph.offset_ref();
    let next_to_port_if_possible = graph
        .get_property_ref(CoreOptions::PORT_LABELS_PLACEMENT)
        .unwrap_or_else(PortLabelPlacement::outside)
        .contains(&PortLabelPlacement::NextToPortIfPossible);

    match ext_side {
        PortSide::North => {
            port_pos.x += padding.left + graph_offset.x - (port_width / 2.0);
            port_pos.y = -port_height - port_offset;
            dummy_guard.shape().position().y = -(padding.top + port_offset + graph_offset.y);
        }
        PortSide::East => {
            let use_compact_zero_width = graph_size.x <= 0.0 && next_to_port_if_possible;
            let horizontal_span = if use_compact_zero_width {
                4.0
            } else {
                graph_size.x + padding.left + padding.right
            };
            port_pos.x = horizontal_span + port_offset;
            port_pos.y += padding.top + graph_offset.y - (port_height / 2.0);
            let dummy_x_span = if use_compact_zero_width {
                horizontal_span
            } else {
                graph_size.x
            };
            dummy_guard.shape().position().x =
                dummy_x_span + padding.right + port_offset - graph_offset.x;
        }
        PortSide::South => {
            port_pos.x += padding.left + graph_offset.x - (port_width / 2.0);
            port_pos.y = graph_size.y + padding.top + padding.bottom + port_offset;
            dummy_guard.shape().position().y =
                graph_size.y + padding.bottom + port_offset - graph_offset.y;
        }
        PortSide::West => {
            port_pos.x = -port_width - port_offset;
            port_pos.y += padding.top + graph_offset.y - (port_height / 2.0);
            dummy_guard.shape().position().x = -(padding.left + port_offset + graph_offset.x);
        }
        PortSide::Undefined => {}
    }

    port_pos
}

/// Sets the size of the given graph such that size constraints are adhered to. Furthermore, the padding is
/// added to the graph size and the graph offset. Afterwards, the border spacing property is reset to 0.
///
/// Major parts of this method are adapted from ElkUtil::resizeNode.
///
/// Note: This method doesn't care about labels of compound nodes since those labels are not attached to the graph.
///
/// # Arguments
/// * `lgraph` - the graph to resize.
fn resize_graph(lgraph: &mut LGraph) {
    let size_constraint = lgraph
        .get_property_ref(LayeredOptions::NODE_SIZE_CONSTRAINTS)
        .unwrap_or_else(EnumSet::none_of);
    let size_options = lgraph
        .get_property_ref(CoreOptions::NODE_SIZE_OPTIONS)
        .unwrap_or_else(EnumSet::none_of);

    // getActualSize() used to take the border spacing (what is now included in the padding)
    // into account, which is why by this point it had to be cleared since it had already
    // been applied to the offset and the graph size. It currently does not take the padding
    // into account anymore, but if it does, it needs to be cleared again
    let calculated_size = lgraph.actual_size();
    let mut adjusted_size = KVector::from_vector(&calculated_size);

    // calculate the new size
    if size_constraint.contains(&SizeConstraint::MinimumSize) {
        let mut min_size = lgraph
            .get_property_ref(LayeredOptions::NODE_SIZE_MINIMUM)
            .unwrap_or_default();

        // if minimum width or height are not set, maybe default to default values
        if size_options.contains(&SizeOptions::DefaultMinimumSize) {
            if min_size.x <= 0.0 {
                min_size.x = ElkUtil::DEFAULT_MIN_WIDTH;
            }

            if min_size.y <= 0.0 {
                min_size.y = ElkUtil::DEFAULT_MIN_HEIGHT;
            }
        }

        // apply new size including border spacing
        adjusted_size.x = adjusted_size.x.max(min_size.x);
        adjusted_size.y = adjusted_size.y.max(min_size.y);
    }

    if std::env::var("ELK_TRACE_SIZING").is_ok() {
        let min_size = lgraph.get_property_ref(LayeredOptions::NODE_SIZE_MINIMUM).unwrap_or_default();
        let padding = lgraph.padding_ref().clone();
        eprintln!("TRACE resize_graph: constraints={:?} calculated_size=({:.1},{:.1}) adjusted_size=({:.1},{:.1}) min_size=({:.1},{:.1}) padding=({:.1},{:.1},{:.1},{:.1})",
            size_constraint, calculated_size.x, calculated_size.y, adjusted_size.x, adjusted_size.y,
            min_size.x, min_size.y, padding.left, padding.top, padding.right, padding.bottom);
    }
    resize_graph_no_really_i_mean_it(lgraph, &calculated_size, &adjusted_size);
}

/// Applies a new effective size to a graph that previously had an old size calculated by the layout algorithm. This
/// method takes care of adjusting content alignments as well as external ports that would be misplaced if the new
/// size is larger than the old one.
///
/// # Arguments
/// * `lgraph` - the graph to apply the size to.
/// * `old_size` - old size as calculated by the layout algorithm.
/// * `new_size` - new size that may be larger than the old one.
fn resize_graph_no_really_i_mean_it(lgraph: &mut LGraph, old_size: &KVector, new_size: &KVector) {
    // obey to specified alignment constraints
    let content_alignment = lgraph
        .get_property_ref(CoreOptions::CONTENT_ALIGNMENT)
        .unwrap_or_else(EnumSet::none_of);

    // horizontal alignment
    if new_size.x > old_size.x {
        if content_alignment.contains(&ContentAlignment::HCenter) {
            lgraph.offset().x += (new_size.x - old_size.x) / 2.0;
        } else if content_alignment.contains(&ContentAlignment::HRight) {
            lgraph.offset().x += new_size.x - old_size.x;
        }
    }

    // vertical alignment
    if new_size.y > old_size.y {
        if content_alignment.contains(&ContentAlignment::VCenter) {
            lgraph.offset().y += (new_size.y - old_size.y) / 2.0;
        } else if content_alignment.contains(&ContentAlignment::VBottom) {
            lgraph.offset().y += new_size.y - old_size.y;
        }
    }

    // correct the position of eastern and southern hierarchical ports, if necessary
    let has_external_ports = lgraph
        .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
        .unwrap_or_else(EnumSet::none_of)
        .contains(&GraphProperties::ExternalPorts);

    if has_external_ports && (new_size.x > old_size.x || new_size.y > old_size.y) {
        // iterate over the graph's nodes, looking for eastern / southern external ports
        // (at this point, the graph's nodes are not divided into layers anymore)
        let layerless_nodes = lgraph.layerless_nodes().clone();
        for node in &layerless_nodes {
            if let Ok(mut node_guard) = node.lock() {
                // we're only looking for external port dummies
                if node_guard.node_type() == NodeType::ExternalPort {
                    // check which side the external port is on
                    let ext_port_side = node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined);

                    let pos = node_guard.shape().position();
                    match ext_port_side {
                        PortSide::East => {
                            pos.x += new_size.x - old_size.x;
                        }
                        PortSide::South => {
                            pos.y += new_size.y - old_size.y;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Actually apply the new size
    let padding = lgraph.padding_ref().clone();
    lgraph.size().x = new_size.x - padding.left - padding.right;
    lgraph.size().y = new_size.y - padding.top - padding.bottom;
}
