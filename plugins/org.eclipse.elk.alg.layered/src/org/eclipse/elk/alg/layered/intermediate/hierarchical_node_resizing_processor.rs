use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::content_alignment::ContentAlignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_options::SizeOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LGraphUtil, LNode, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{GraphProperties, InternalProperties, LayeredOptions};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;

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

        // Clear layer assignments for all layerless nodes
        for node in graph.layerless_nodes().clone() {
            LNode::set_layer(&node, None);
        }

        // Clear all layers
        graph.layers_mut().clear();

        // Resize the graph
        resize_graph(graph);

        // If nested, transfer layout to parent node
        if is_nested(graph) {
            if let Some(parent_node) = graph.parent_node() {
                graph_layout_to_node(&parent_node, graph);
            }
        }

        progress_monitor.done();
    }
}

/// Transfer the layout of the given graph to the given associated node.
///
/// # Arguments
/// * `node` - a compound node
/// * `lgraph` - the graph nested in the compound node
fn graph_layout_to_node(node: &LNodeRef, lgraph: &mut LGraph) {
    // Process external ports
    let layerless_nodes = lgraph.layerless_nodes().clone();
    for child_node in &layerless_nodes {
        if let Ok(mut child_guard) = child_node.lock() {
            if let Some(Origin::LPort(port_ref)) = child_guard.get_property(InternalProperties::ORIGIN) {
                // Get port size and external port side
                let port_size = if let Ok(mut port_guard) = port_ref.lock() {
                    *port_guard.shape().size_ref()
                } else {
                    continue;
                };

                let ext_port_side = child_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined);

                // Get the graph reference
                let graph_ref = if let Some(graph) = child_guard.graph() {
                    graph
                } else {
                    continue;
                };

                let port_position = LGraphUtil::get_external_port_position(
                    &graph_ref,
                    child_node,
                    port_size.x,
                    port_size.y,
                );

                // Set port position and side
                if let Ok(mut port_guard) = port_ref.lock() {
                    let pos = port_guard.shape().position();
                    pos.x = port_position.x;
                    pos.y = port_position.y;
                    port_guard.set_side(ext_port_side);
                }
            }
        }
    }

    // Setup the parent node
    let actual_graph_size = lgraph.actual_size();
    let has_external_ports = lgraph
        .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
        .unwrap_or_else(|| EnumSet::none_of())
        .contains(&GraphProperties::ExternalPorts);

    if let Ok(mut node_guard) = node.lock() {
        if has_external_ports {
            // Ports have positions assigned
            node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedPos));

            // Add NON_FREE_PORTS to the parent graph's properties
            if let Some(parent_graph_ref) = node_guard.graph() {
                if let Ok(mut parent_graph_guard) = parent_graph_ref.lock() {
                    let mut graph_props = parent_graph_guard
                        .get_property(InternalProperties::GRAPH_PROPERTIES)
                        .unwrap_or_else(|| EnumSet::none_of());
                    graph_props.insert(GraphProperties::NonFreePorts);
                    parent_graph_guard.set_property(
                        InternalProperties::GRAPH_PROPERTIES,
                        Some(graph_props),
                    );
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
        .unwrap_or_else(|| EnumSet::none_of());
    let size_options = lgraph
        .get_property_ref(CoreOptions::NODE_SIZE_OPTIONS)
        .unwrap_or_else(|| EnumSet::none_of());

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
            .unwrap_or_else(KVector::new);

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
        .unwrap_or_else(|| EnumSet::none_of());

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
        .unwrap_or_else(|| EnumSet::none_of())
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
