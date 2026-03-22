use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, Spacings,
};

pub struct CommentNodeMarginCalculator;

impl ILayoutProcessor<LGraph> for CommentNodeMarginCalculator {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node margin calculation", 1.0);

        for layer in layered_graph.layers().clone() {
            let nodes = layer
                .lock().nodes().clone();
            for node in nodes {
                process_comments(layered_graph, &node);
            }
        }

        monitor.done();
    }
}

fn process_comments(layered_graph: &LGraph, node: &LNodeRef) {
    let (top_boxes, bottom_boxes, node_width) = if let Some(mut node_guard) = node.lock_ok() {
        let top_boxes = if node_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(InternalProperties::TOP_COMMENTS)
        {
            node_guard.get_property(InternalProperties::TOP_COMMENTS)
        } else {
            None
        };
        let bottom_boxes = if node_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(InternalProperties::BOTTOM_COMMENTS)
        {
            node_guard.get_property(InternalProperties::BOTTOM_COMMENTS)
        } else {
            None
        };
        (top_boxes, bottom_boxes, node_guard.shape().size_ref().x)
    } else {
        return;
    };

    if top_boxes.is_none() && bottom_boxes.is_none() {
        return;
    }

    let comment_comment_spacing = Spacings::get_individual_or_default_with_graph(
        layered_graph,
        node,
        LayeredOptions::SPACING_COMMENT_COMMENT,
    );
    let comment_node_spacing = Spacings::get_individual_or_default_with_graph(
        layered_graph,
        node,
        LayeredOptions::SPACING_COMMENT_NODE,
    );

    let mut top_width: f64 = 0.0;
    let mut top_extra_margin = 0.0;
    if let Some(top_boxes) = top_boxes {
        if !top_boxes.is_empty() {
            let mut max_height: f64 = 0.0;
            for comment_box in &top_boxes {
                {
                    let mut comment_guard = comment_box.lock();
                    let size = comment_guard.shape().size_ref();
                    max_height = max_height.max(size.y);
                    top_width += size.x;
                }
            }
            top_width += comment_comment_spacing * (top_boxes.len().saturating_sub(1) as f64);
            top_extra_margin = max_height + comment_node_spacing;
        }
    }

    let mut bottom_width: f64 = 0.0;
    let mut bottom_extra_margin = 0.0;
    if let Some(bottom_boxes) = bottom_boxes {
        if !bottom_boxes.is_empty() {
            let mut max_height: f64 = 0.0;
            for comment_box in &bottom_boxes {
                {
                    let mut comment_guard = comment_box.lock();
                    let size = comment_guard.shape().size_ref();
                    max_height = max_height.max(size.y);
                    bottom_width += size.x;
                }
            }
            bottom_width += comment_comment_spacing * (bottom_boxes.len().saturating_sub(1) as f64);
            bottom_extra_margin = max_height + comment_node_spacing;
        }
    }

    {
        let mut node_guard = node.lock();
        let margin = node_guard.margin();
        margin.top += top_extra_margin;
        margin.bottom += bottom_extra_margin;

        let max_comment_width = top_width.max(bottom_width);
        if max_comment_width > node_width {
            let protrusion = (max_comment_width - node_width) / 2.0;
            margin.left = margin.left.max(protrusion);
            margin.right = margin.right.max(protrusion);
        }
    }
}
