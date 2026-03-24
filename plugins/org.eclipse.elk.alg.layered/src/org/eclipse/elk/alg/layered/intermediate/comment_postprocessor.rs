use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct CommentPostprocessor;

impl ILayoutProcessor<LGraph> for CommentPostprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Comment post-processing", 1.0);

        let graph_comment_spacing = if layered_graph
            .graph_element()
            .properties()
            .has_property(LayeredOptions::SPACING_COMMENT_COMMENT)
        {
            layered_graph
                .get_property(LayeredOptions::SPACING_COMMENT_COMMENT)
                .unwrap_or(10.0)
        } else {
            10.0
        };

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = {
                let layer_guard = layer.lock();
                layer_guard.nodes().clone()
            };
            let mut comment_boxes = Vec::new();

            for node in nodes {
                let (top_boxes, bottom_boxes) = {
                    let mut node_guard = node.lock();
                    let top = if node_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(InternalProperties::TOP_COMMENTS)
                    {
                        node_guard.get_property(InternalProperties::TOP_COMMENTS)
                    } else {
                        None
                    };
                    let bottom = if node_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(InternalProperties::BOTTOM_COMMENTS)
                    {
                        node_guard.get_property(InternalProperties::BOTTOM_COMMENTS)
                    } else {
                        None
                    };
                    (top, bottom)
                };

                if top_boxes.is_none() && bottom_boxes.is_none() {
                    continue;
                }

                process_node(
                    &node,
                    top_boxes.as_deref(),
                    bottom_boxes.as_deref(),
                    graph_comment_spacing,
                );
                if let Some(top_boxes) = top_boxes {
                    comment_boxes.extend(top_boxes);
                }
                if let Some(bottom_boxes) = bottom_boxes {
                    comment_boxes.extend(bottom_boxes);
                }
            }

            {
                let mut layer_guard = layer.lock();
                layer_guard.nodes_mut().extend(comment_boxes);
            }
        }

        monitor.done();
    }
}

fn process_node(
    node: &LNodeRef,
    top_boxes: Option<&[LNodeRef]>,
    bottom_boxes: Option<&[LNodeRef]>,
    graph_comment_spacing: f64,
) {
    let (node_x, node_y, node_w, node_h, margin_top, margin_bottom) = {
        let mut node_guard = node.lock();
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().position_ref().y,
            node_guard.shape().size_ref().x,
            node_guard.shape().size_ref().y,
            node_guard.margin().top,
            node_guard.margin().bottom,
        )
    };

    let comment_comment_spacing = spacing_comment_comment(node, graph_comment_spacing);

    if let Some(top_boxes) = top_boxes {
        let mut boxes_width = comment_comment_spacing * (top_boxes.len().saturating_sub(1) as f64);
        let mut max_height: f64 = 0.0;
        for box_node in top_boxes {
            {
                let mut box_guard = box_node.lock();
                boxes_width += box_guard.shape().size_ref().x;
                max_height = max_height.max(box_guard.shape().size_ref().y);
            }
        }

        let mut x = node_x - (boxes_width - node_w) / 2.0;
        let base_line = node_y - margin_top + max_height;
        let anchor_inc = node_w / ((top_boxes.len() + 1) as f64);
        let mut anchor_x = anchor_inc;
        for box_node in top_boxes {
            {
                let mut box_guard = box_node.lock();
                box_guard.shape().position().x = x;
                box_guard.shape().position().y = base_line - box_guard.shape().size_ref().y;
                x += box_guard.shape().size_ref().x + comment_comment_spacing;
            }

            if let Some(box_port) = get_box_port(box_node) {
                let mut box_port_guard = box_port.lock();
                let box_size_y = {
                    let mut box_guard = box_node.lock();
                    box_guard.shape().size_ref().y
                };
                let box_size_x = {
                    let mut box_guard = box_node.lock();
                    box_guard.shape().size_ref().x
                };
                let anchor_x_offset = box_port_guard.anchor_ref().x;
                box_port_guard.shape().position().x = box_size_x / 2.0 - anchor_x_offset;
                box_port_guard.shape().position().y = box_size_y;
            }

            if let Some(node_port) = comment_conn_port(box_node) {
                let mut attach = false;
                {
                    let mut node_port_guard = node_port.lock();
                    if node_port_guard.degree() == 1 {
                        let anchor_x_offset = node_port_guard.anchor_ref().x;
                        node_port_guard.shape().position().x = anchor_x - anchor_x_offset;
                        node_port_guard.shape().position().y = 0.0;
                        attach = true;
                    }
                }
                if attach {
                    crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(
                        &node_port,
                        Some(node.clone()),
                    );
                }
            }

            anchor_x += anchor_inc;
        }
    }

    if let Some(bottom_boxes) = bottom_boxes {
        let mut boxes_width =
            comment_comment_spacing * (bottom_boxes.len().saturating_sub(1) as f64);
        let mut max_height: f64 = 0.0;
        for box_node in bottom_boxes {
            {
                let mut box_guard = box_node.lock();
                boxes_width += box_guard.shape().size_ref().x;
                max_height = max_height.max(box_guard.shape().size_ref().y);
            }
        }

        let mut x = node_x - (boxes_width - node_w) / 2.0;
        let base_line = node_y + node_h + margin_bottom - max_height;
        let anchor_inc = node_w / ((bottom_boxes.len() + 1) as f64);
        let mut anchor_x = anchor_inc;
        for box_node in bottom_boxes {
            {
                let mut box_guard = box_node.lock();
                box_guard.shape().position().x = x;
                box_guard.shape().position().y = base_line;
                x += box_guard.shape().size_ref().x + comment_comment_spacing;
            }

            if let Some(box_port) = get_box_port(box_node) {
                let mut box_port_guard = box_port.lock();
                let box_size_x = {
                    let mut box_guard = box_node.lock();
                    box_guard.shape().size_ref().x
                };
                let anchor_x_offset = box_port_guard.anchor_ref().x;
                box_port_guard.shape().position().x = box_size_x / 2.0 - anchor_x_offset;
                box_port_guard.shape().position().y = 0.0;
            }

            if let Some(node_port) = comment_conn_port(box_node) {
                let mut attach = false;
                {
                    let mut node_port_guard = node_port.lock();
                    if node_port_guard.degree() == 1 {
                        let anchor_x_offset = node_port_guard.anchor_ref().x;
                        node_port_guard.shape().position().x = anchor_x - anchor_x_offset;
                        node_port_guard.shape().position().y = node_h;
                        attach = true;
                    }
                }
                if attach {
                    crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(
                        &node_port,
                        Some(node.clone()),
                    );
                }
            }

            anchor_x += anchor_inc;
        }
    }
}

fn get_box_port(comment_box: &LNodeRef) -> Option<LPortRef> {
    let node_port = comment_conn_port(comment_box)?;
    let ports = {
        let box_guard = comment_box.lock();
        box_guard.ports().clone()
    };
    for port in ports {
        let outgoing_edges = {
            let port_guard = port.lock();
            port_guard.outgoing_edges().clone()
        };
        if let Some(edge) = outgoing_edges.into_iter().next() {
            LEdge::set_target(&edge, Some(node_port.clone()));
            return Some(port);
        }

        let incoming_edges = {
            let port_guard = port.lock();
            port_guard.incoming_edges().clone()
        };
        if let Some(edge) = incoming_edges.into_iter().next() {
            LEdge::set_source(&edge, Some(node_port.clone()));
            return Some(port);
        }
    }
    None
}

fn comment_conn_port(comment_box: &LNodeRef) -> Option<LPortRef> {
    let mut box_guard = comment_box.lock();
    if box_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(InternalProperties::COMMENT_CONN_PORT)
    {
        box_guard.get_property(InternalProperties::COMMENT_CONN_PORT)
    } else {
        None
    }
}

fn spacing_comment_comment(node: &LNodeRef, graph_comment_spacing: f64) -> f64 {
    let mut node_guard = node.lock();
    let has_individual = node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(CoreOptions::SPACING_INDIVIDUAL);
    if has_individual {
        if let Some(mut individual) = node_guard.get_property(CoreOptions::SPACING_INDIVIDUAL) {
            if individual
                .properties()
                .has_property(LayeredOptions::SPACING_COMMENT_COMMENT)
            {
                if let Some(value) = individual
                    .properties_mut()
                    .get_property(LayeredOptions::SPACING_COMMENT_COMMENT)
                {
                    return value;
                }
            }
        }
    }

    graph_comment_spacing
}
