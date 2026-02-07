use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{
    EdgeLabelSideSelection, InternalProperties, LayeredOptions,
};

pub struct LabelSideSelector;

impl ILayoutProcessor<LGraph> for LabelSideSelector {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        let mode = layered_graph
            .get_property(LayeredOptions::EDGE_LABELS_SIDE_SELECTION)
            .unwrap_or(EdgeLabelSideSelection::SmartDown);
        monitor.begin("Label side selection", 1.0);

        let default_side = default_side_for(mode);
        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                apply_label_side_to_dummy(&node, default_side);
                apply_label_side_to_outgoing_edges(&node, mode, default_side);
            }
        }

        monitor.done();
    }
}

fn default_side_for(mode: EdgeLabelSideSelection) -> LabelSide {
    match mode {
        EdgeLabelSideSelection::AlwaysUp
        | EdgeLabelSideSelection::DirectionUp
        | EdgeLabelSideSelection::SmartUp => LabelSide::Above,
        EdgeLabelSideSelection::AlwaysDown
        | EdgeLabelSideSelection::DirectionDown
        | EdgeLabelSideSelection::SmartDown => LabelSide::Below,
    }
}

fn apply_label_side_to_dummy(node: &LNodeRef, default_side: LabelSide) {
    let is_label_dummy = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.node_type() == NodeType::Label)
        .unwrap_or(false);
    if !is_label_dummy {
        return;
    }

    let effective_side = node
        .lock()
        .ok()
        .map(|mut node_guard| {
            if node_guard.is_inline_edge_label() {
                LabelSide::Inline
            } else {
                default_side
            }
        })
        .unwrap_or(default_side);

    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_property(InternalProperties::LABEL_SIDE, Some(effective_side));
        let represented = node_guard
            .get_property(InternalProperties::REPRESENTED_LABELS)
            .unwrap_or_default();
        for label in represented {
            if let Ok(mut label_guard) = label.lock() {
                label_guard.set_property(InternalProperties::LABEL_SIDE, Some(effective_side));
            }
        }
    }
}

fn apply_label_side_to_outgoing_edges(
    node: &LNodeRef,
    mode: EdgeLabelSideSelection,
    default_side: LabelSide,
) {
    let outgoing = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.outgoing_edges())
        .unwrap_or_default();
    for edge in outgoing {
        let edge_points_right = true;
        let side = match mode {
            EdgeLabelSideSelection::DirectionUp | EdgeLabelSideSelection::DirectionDown => {
                if edge_points_right {
                    default_side
                } else {
                    default_side.opposite()
                }
            }
            _ => default_side,
        };

        let labels = edge
            .lock()
            .ok()
            .map(|edge_guard| edge_guard.labels().clone())
            .unwrap_or_default();
        for label in labels {
            if let Ok(mut label_guard) = label.lock() {
                label_guard.set_property(InternalProperties::LABEL_SIDE, Some(side));
            }
        }
    }
}
