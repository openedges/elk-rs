use std::collections::VecDeque;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LLabelRef, LNodeRef, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeLabelSideSelection, InternalProperties, LayeredOptions,
};

use super::end_label_preprocessor::gather_labels;

pub struct LabelSideSelector;

impl ILayoutProcessor<LGraph> for LabelSideSelector {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        let mode = layered_graph
            .get_property(LayeredOptions::EDGE_LABELS_SIDE_SELECTION)
            .unwrap_or(EdgeLabelSideSelection::SmartDown);
        let edge_label_spacing = if layered_graph
            .graph_element()
            .properties()
            .has_property(LayeredOptions::SPACING_EDGE_LABEL)
        {
            layered_graph
                .get_property(LayeredOptions::SPACING_EDGE_LABEL)
                .unwrap_or(2.0)
        } else {
            2.0
        };
        monitor.begin("Label side selection", 1.0);

        match mode {
            EdgeLabelSideSelection::AlwaysUp => {
                same_side(layered_graph, LabelSide::Above, edge_label_spacing)
            }
            EdgeLabelSideSelection::AlwaysDown => {
                same_side(layered_graph, LabelSide::Below, edge_label_spacing)
            }
            EdgeLabelSideSelection::DirectionUp => {
                based_on_direction(layered_graph, LabelSide::Above, edge_label_spacing)
            }
            EdgeLabelSideSelection::DirectionDown => {
                based_on_direction(layered_graph, LabelSide::Below, edge_label_spacing)
            }
            EdgeLabelSideSelection::SmartUp => {
                smart(layered_graph, LabelSide::Above, edge_label_spacing)
            }
            EdgeLabelSideSelection::SmartDown => {
                smart(layered_graph, LabelSide::Below, edge_label_spacing)
            }
        }

        monitor.done();
    }
}

// ========================================================================================
// Simple Placement Strategies
// ========================================================================================

/// Configures all labels to be placed on the given side.
fn same_side(graph: &LGraph, label_side: LabelSide, edge_label_spacing: f64) {
    let layers = graph.layers().clone();
    for layer in layers {
        let nodes = layer
            .lock().nodes().clone();
        for node in nodes {
            apply_label_side_to_dummy(&node, label_side, edge_label_spacing);

            let outgoing = node
                .lock().outgoing_edges();
            for edge in outgoing {
                apply_label_side_to_edge(&edge, label_side);
            }
        }
    }
}

/// Configures all labels to be placed according to their edge's direction.
fn based_on_direction(
    graph: &LGraph,
    side_for_rightward_edges: LabelSide,
    edge_label_spacing: f64,
) {
    let layers = graph.layers().clone();
    for layer in layers {
        let nodes = layer
            .lock().nodes().clone();
        for node in nodes {
            let is_label = node.lock().node_type() == NodeType::Label;
            if is_label {
                let side = if does_edge_point_right_node(&node) {
                    side_for_rightward_edges
                } else {
                    side_for_rightward_edges.opposite()
                };
                apply_label_side_to_dummy(&node, side, edge_label_spacing);
            }

            let outgoing = node
                .lock().outgoing_edges();
            for edge in outgoing {
                let side = if does_edge_point_right_edge(&edge) {
                    side_for_rightward_edges
                } else {
                    side_for_rightward_edges.opposite()
                };
                apply_label_side_to_edge(&edge, side);
            }
        }
    }
}

// ========================================================================================
// Smart Placement Strategy
// ========================================================================================

/// Chooses label sides depending on certain patterns. If in doubt, uses the given default side.
fn smart(graph: &LGraph, default_side: LabelSide, edge_label_spacing: f64) {
    let mut dummy_node_queue: VecDeque<LNodeRef> = VecDeque::new();

    let layers = graph.layers().clone();
    for layer in layers {
        let mut top_group = true;
        let mut label_dummies_in_queue = 0;

        let nodes = layer
            .lock().nodes().clone();

        for node in &nodes {
            let node_type = node
                .lock().node_type();

            match node_type {
                NodeType::Label => {
                    label_dummies_in_queue += 1;
                    // Fall through to add to queue
                    dummy_node_queue.push_back(node.clone());
                }
                NodeType::LongEdge => {
                    dummy_node_queue.push_back(node.clone());
                }
                _ => {
                    if node_type == NodeType::Normal {
                        smart_for_regular_node(node, default_side);
                    }

                    // Empty dummy node queue (handles NORMAL fall-through and default)
                    if !dummy_node_queue.is_empty() {
                        smart_for_consecutive_dummy_node_run(
                            &mut dummy_node_queue,
                            label_dummies_in_queue,
                            top_group,
                            false,
                            default_side,
                            edge_label_spacing,
                        );
                    }

                    // Reset things
                    top_group = false;
                    label_dummies_in_queue = 0;
                }
            }
        }

        // Do stuff with the nodes remaining in the queue
        if !dummy_node_queue.is_empty() {
            smart_for_consecutive_dummy_node_run(
                &mut dummy_node_queue,
                label_dummies_in_queue,
                top_group,
                true,
                default_side,
                edge_label_spacing,
            );
        }
    }
}

/// Assigns label sides to all label dummies in the given queue and empties the queue afterwards.
fn smart_for_consecutive_dummy_node_run(
    dummy_nodes: &mut VecDeque<LNodeRef>,
    label_dummy_count: usize,
    top_group: bool,
    bottom_group: bool,
    default_side: LabelSide,
    edge_label_spacing: f64,
) {
    assert!(!dummy_nodes.is_empty());

    let first_is_label = dummy_nodes
        .front()
        .map(|n| n.lock().node_type() == NodeType::Label)
        .unwrap_or(false);
    let last_is_label = dummy_nodes
        .back()
        .map(|n| n.lock().node_type() == NodeType::Label)
        .unwrap_or(false);

    if top_group
        && (!bottom_group || dummy_nodes.len() > 1)
        && label_dummy_count == 1
        && first_is_label
    {
        // Top of layer with single label dummy at top -> ABOVE
        if let Some(front) = dummy_nodes.front() {
            apply_label_side_to_dummy(front, LabelSide::Above, edge_label_spacing);
        }
    } else if bottom_group
        && (!top_group || dummy_nodes.len() > 1)
        && label_dummy_count == 1
        && last_is_label
    {
        // Bottom of layer with single label dummy at bottom -> BELOW
        if let Some(back) = dummy_nodes.back() {
            apply_label_side_to_dummy(back, LabelSide::Below, edge_label_spacing);
        }
    } else if dummy_nodes.len() == 2 {
        // Two-node run: first ABOVE, second BELOW
        if let Some(first) = dummy_nodes.pop_front() {
            apply_label_side_to_dummy(&first, LabelSide::Above, edge_label_spacing);
        }
        if let Some(second) = dummy_nodes.pop_front() {
            apply_label_side_to_dummy(&second, LabelSide::Below, edge_label_spacing);
        }
    } else {
        // Not a special case: check for simple loops
        apply_for_dummy_node_run_with_simple_loops(
            dummy_nodes,
            label_dummy_count,
            default_side,
            edge_label_spacing,
        );
    }

    dummy_nodes.clear();
}

/// Takes a collection of dummy nodes and applies label sides. Detects simple loops (two
/// consecutive label dummies connecting the same nodes).
fn apply_for_dummy_node_run_with_simple_loops(
    dummy_nodes: &VecDeque<LNodeRef>,
    _label_dummy_count: usize,
    default_side: LabelSide,
    edge_label_spacing: f64,
) {
    let mut label_dummy_run: Vec<LNodeRef> = Vec::with_capacity(dummy_nodes.len());
    let mut prev_long_edge_source: Option<LNodeRef> = None;
    let mut prev_long_edge_target: Option<LNodeRef> = None;

    for current_dummy in dummy_nodes {
        let curr_long_edge_source = get_long_edge_end_node(current_dummy, true);
        let curr_long_edge_target = get_long_edge_end_node(current_dummy, false);

        let same_source = match (&prev_long_edge_source, &curr_long_edge_source) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let same_target = match (&prev_long_edge_target, &curr_long_edge_target) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if !same_source || !same_target {
            // Starting a new run
            apply_label_sides_to_label_dummy_run(
                &mut label_dummy_run,
                default_side,
                edge_label_spacing,
            );
            prev_long_edge_source = curr_long_edge_source;
            prev_long_edge_target = curr_long_edge_target;
        }

        label_dummy_run.push(current_dummy.clone());
    }

    // Assign label sides to whatever dummy nodes are left
    apply_label_sides_to_label_dummy_run(&mut label_dummy_run, default_side, edge_label_spacing);
}

/// Returns either the long edge source or target node of the given dummy node.
fn get_long_edge_end_node(dummy: &LNodeRef, source: bool) -> Option<LNodeRef> {
    let port: Option<LPortRef> = {
        let ng = dummy.lock();
        if source {
            ng.get_property(InternalProperties::LONG_EDGE_SOURCE)
        } else {
            ng.get_property(InternalProperties::LONG_EDGE_TARGET)
        }
    };

    port.and_then(|p| p.lock().node())
}

/// Applies label sides to the given list of consecutive dummy nodes and empties the list.
fn apply_label_sides_to_label_dummy_run(
    label_dummy_run: &mut Vec<LNodeRef>,
    default_side: LabelSide,
    edge_label_spacing: f64,
) {
    if !label_dummy_run.is_empty() {
        if label_dummy_run.len() == 2 {
            apply_label_side_to_dummy(&label_dummy_run[0], LabelSide::Above, edge_label_spacing);
            apply_label_side_to_dummy(&label_dummy_run[1], LabelSide::Below, edge_label_spacing);
        } else {
            for dummy_node in label_dummy_run.iter() {
                apply_label_side_to_dummy(dummy_node, default_side, edge_label_spacing);
            }
        }
        label_dummy_run.clear();
    }
}

/// Assigns label sides to all end labels incident to this node.
fn smart_for_regular_node(node: &LNodeRef, default_side: LabelSide) {
    let ports = node
        .lock().ports().clone();

    let mut end_label_queue: VecDeque<Vec<LLabelRef>> = VecDeque::with_capacity(ports.len());
    let mut current_port_side: Option<PortSide> = None;

    for port in &ports {
        let port_side = port
            .lock().side();

        if Some(port_side) != current_port_side {
            if !end_label_queue.is_empty() {
                if let Some(cps) = current_port_side {
                    smart_for_regular_node_port_end_labels(&mut end_label_queue, cps, default_side);
                }
            }
            end_label_queue.clear();
            current_port_side = Some(port_side);
        }

        // Possibly add the port's end labels to our queue
        if let Some(port_end_labels) = gather_labels(port) {
            end_label_queue.push_back(port_end_labels);
        }
    }

    // Clear remaining ports
    if !end_label_queue.is_empty() {
        if let Some(cps) = current_port_side {
            smart_for_regular_node_port_end_labels(&mut end_label_queue, cps, default_side);
        }
    }
}

/// Handle the end labels currently in the queue.
fn smart_for_regular_node_port_end_labels(
    end_label_queue: &mut VecDeque<Vec<LLabelRef>>,
    port_side: PortSide,
    default_side: LabelSide,
) {
    assert!(!end_label_queue.is_empty());

    if end_label_queue.len() == 2 {
        if port_side == PortSide::North || port_side == PortSide::East {
            if let Some(first) = end_label_queue.pop_front() {
                apply_label_side_to_labels(&first, LabelSide::Above);
            }
            if let Some(second) = end_label_queue.pop_front() {
                apply_label_side_to_labels(&second, LabelSide::Below);
            }
        } else {
            if let Some(first) = end_label_queue.pop_front() {
                apply_label_side_to_labels(&first, LabelSide::Below);
            }
            if let Some(second) = end_label_queue.pop_front() {
                apply_label_side_to_labels(&second, LabelSide::Above);
            }
        }
    } else {
        for label_list in end_label_queue.iter() {
            apply_label_side_to_labels(label_list, default_side);
        }
    }
}

// ========================================================================================
// Helper Methods
// ========================================================================================

/// Applies the given label side to the given label dummy node. If necessary, its ports are
/// moved to reserve space for the label on the correct side.
fn apply_label_side_to_dummy(node: &LNodeRef, side: LabelSide, edge_label_spacing: f64) {
    let is_label_dummy = node.lock().node_type() == NodeType::Label;
    if !is_label_dummy {
        return;
    }

    let effective_side = {
        let mut node_guard = node.lock();
        if node_guard.is_inline_edge_label() {
            LabelSide::Inline
        } else {
            side
        }
    };

    {
        let mut node_guard = node.lock();
        node_guard.set_property(InternalProperties::LABEL_SIDE, Some(effective_side));
        let represented = node_guard
            .get_property(InternalProperties::REPRESENTED_LABELS)
            .unwrap_or_default();
        for label in represented {
            {
                let mut label_guard = label.lock();
                label_guard.set_property(InternalProperties::LABEL_SIDE, Some(effective_side));
            }
        }
    }

    // If the label is not below the edge, the ports need to be moved
    if effective_side != LabelSide::Below {
        // Get edge thickness from one of the incident edges
        let thickness = get_origin_edge_thickness(node);

        let mut port_pos: f64 = 0.0;
        if effective_side == LabelSide::Above {
            let node_height = node.lock().shape().size_ref().y;
            port_pos = node_height - (thickness / 2.0).ceil();
        } else if effective_side == LabelSide::Inline {
            let node_height = node.lock().shape().size_ref().y;

            port_pos = (node_height - edge_label_spacing - thickness).ceil() / 2.0;

            // Reduce size of the label dummy
            {
                let mut ng = node.lock();
                let new_y = ng.shape().size_ref().y - edge_label_spacing - thickness;
                ng.shape().size().y = new_y;
            }
        }

        // Move all ports
        let ports = node
            .lock().ports().clone();
        for port in ports {
            {
                let mut pg = port.lock();
                pg.shape().position().y = port_pos;
            }
        }
    }
}

/// Gets the edge thickness for a label dummy node by looking at the incoming edge's
/// EDGE_THICKNESS property (the label dummy's incoming edge is part of the long edge
/// that this label represents).
fn get_origin_edge_thickness(label_dummy: &LNodeRef) -> f64 {
    // Try to get the edge from the incoming edges
    let incoming = label_dummy
        .lock().incoming_edges();
    if let Some(edge) = incoming.first() {
        {
            let eg = edge.lock();
            return eg.get_property(CoreOptions::EDGE_THICKNESS).unwrap_or(0.0);
        }
    }
    // Fall back to outgoing
    let outgoing = label_dummy
        .lock().outgoing_edges();
    if let Some(edge) = outgoing.first() {
        {
            let eg = edge.lock();
            return eg.get_property(CoreOptions::EDGE_THICKNESS).unwrap_or(0.0);
        }
    }
    0.0
}

/// Applies the given label side to all labels of the given edge.
fn apply_label_side_to_edge(edge: &LEdgeRef, side: LabelSide) {
    let labels = edge
        .lock().labels().clone();
    for label in labels {
        {
            let mut label_guard = label.lock();
            label_guard.set_property(InternalProperties::LABEL_SIDE, Some(side));
        }
    }
}

/// Applies the given label side to all labels in the list.
fn apply_label_side_to_labels(labels: &[LLabelRef], side: LabelSide) {
    for label in labels {
        {
            let mut label_guard = label.lock();
            label_guard.set_property(InternalProperties::LABEL_SIDE, Some(side));
        }
    }
}

/// Checks if the given edge will point right in the final drawing.
fn does_edge_point_right_edge(edge: &LEdgeRef) -> bool {
    !edge
        .lock()
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false)
}

/// Checks if the given label dummy node is part of an edge segment that will point right.
fn does_edge_point_right_node(label_dummy: &LNodeRef) -> bool {
    let incoming = label_dummy
        .lock().incoming_edges();
    let outgoing = label_dummy
        .lock().outgoing_edges();

    let incoming_right = incoming
        .first()
        .map(does_edge_point_right_edge)
        .unwrap_or(false);
    let outgoing_right = outgoing
        .first()
        .map(does_edge_point_right_edge)
        .unwrap_or(false);

    incoming_right || outgoing_right
}
