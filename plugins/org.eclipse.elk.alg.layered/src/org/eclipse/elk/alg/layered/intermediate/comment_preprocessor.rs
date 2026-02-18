use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LNode, LNodeRef, LPortRef,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;

pub struct CommentPreprocessor;

impl ILayoutProcessor<LGraph> for CommentPreprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Comment pre-processing", 1.0);

        let mut index = 0;
        while index < layered_graph.layerless_nodes().len() {
            let node = layered_graph.layerless_nodes()[index].clone();
            if !is_comment_box(&node) {
                index += 1;
                continue;
            }

            let mut edge_count = 0_usize;
            let mut edge: Option<LEdgeRef> = None;
            let mut opposite_port: Option<LPortRef> = None;
            let ports = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for port in ports {
                let (degree, incoming, outgoing) = if let Ok(port_guard) = port.lock() {
                    (
                        port_guard.degree(),
                        port_guard.incoming_edges().clone(),
                        port_guard.outgoing_edges().clone(),
                    )
                } else {
                    continue;
                };
                edge_count += degree;
                if let Some(incoming_edge) = incoming.first() {
                    if incoming.len() == 1 {
                        edge = Some(incoming_edge.clone());
                        opposite_port = incoming_edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.source());
                    }
                }
                if let Some(outgoing_edge) = outgoing.first() {
                    if outgoing.len() == 1 {
                        edge = Some(outgoing_edge.clone());
                        opposite_port = outgoing_edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.target());
                    }
                }
            }

            let qualifies = if let Some(opposite_port) = &opposite_port {
                let opposite_node = opposite_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                edge_count == 1
                    && opposite_port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.degree() == 1)
                        .unwrap_or(false)
                    && opposite_node
                        .as_ref()
                        .map(|node_ref| !is_comment_box(node_ref))
                        .unwrap_or(false)
            } else {
                false
            };

            if qualifies {
                if let (Some(edge), Some(opposite_port)) = (edge, opposite_port) {
                    let real_node = opposite_port
                        .lock()
                        .ok()
                        .and_then(|port_guard| port_guard.node());
                    if let Some(real_node) = real_node {
                        process_box(&node, &edge, &opposite_port, &real_node);
                        layered_graph.layerless_nodes_mut().remove(index);
                        continue;
                    }
                }
            } else {
                reverse_oddly_connected_edges(&node);
            }

            index += 1;
        }

        monitor.done();
    }
}

fn is_comment_box(node: &LNodeRef) -> bool {
    node.lock()
        .ok()
        .and_then(|mut node_guard| {
            if node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(LayeredOptions::COMMENT_BOX)
            {
                node_guard.get_property(LayeredOptions::COMMENT_BOX)
            } else {
                None
            }
        })
        .unwrap_or(false)
}

fn reverse_oddly_connected_edges(comment_node: &LNodeRef) {
    let ports = comment_node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    let mut reversed_edges = Vec::<LEdgeRef>::new();
    for port in ports {
        let (outgoing_edges, incoming_edges) = if let Ok(port_guard) = port.lock() {
            (
                port_guard.outgoing_edges().clone(),
                port_guard.incoming_edges().clone(),
            )
        } else {
            continue;
        };

        for out_edge in outgoing_edges {
            let odd = out_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|target_port| {
                    target_port
                        .lock()
                        .ok()
                        .map(|target_guard| !target_guard.outgoing_edges().is_empty())
                })
                .unwrap_or(false);
            if odd {
                reversed_edges.push(out_edge);
            }
        }

        for in_edge in incoming_edges {
            let odd = in_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source())
                .and_then(|source_port| {
                    source_port
                        .lock()
                        .ok()
                        .map(|source_guard| !source_guard.incoming_edges().is_empty())
                })
                .unwrap_or(false);
            if odd {
                reversed_edges.push(in_edge);
            }
        }
    }

    let layered_graph = comment_node
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.graph());
    if let Some(layered_graph) = layered_graph {
        for edge in reversed_edges {
            LEdge::reverse(&edge, &layered_graph, true);
        }
    }
}

fn process_box(
    box_node: &LNodeRef,
    edge: &LEdgeRef,
    opposite_port: &LPortRef,
    real_node: &LNodeRef,
) {
    let (only_top, only_bottom, top_first) = choose_comment_side(real_node);

    if let Ok(mut real_node_guard) = real_node.lock() {
        let has_top = real_node_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(InternalProperties::TOP_COMMENTS);
        let has_bottom = real_node_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(InternalProperties::BOTTOM_COMMENTS);
        let mut top_boxes = if has_top {
            real_node_guard
                .get_property(InternalProperties::TOP_COMMENTS)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut bottom_boxes = if has_bottom {
            real_node_guard
                .get_property(InternalProperties::BOTTOM_COMMENTS)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let use_top = if top_first {
            if !has_top || only_top {
                true
            } else if !has_bottom {
                false
            } else {
                top_boxes.len() <= bottom_boxes.len()
            }
        } else if !has_bottom || only_bottom {
            false
        } else if !has_top {
            true
        } else {
            bottom_boxes.len() > top_boxes.len()
        };

        if use_top {
            top_boxes.push(box_node.clone());
            real_node_guard.set_property(InternalProperties::TOP_COMMENTS, Some(top_boxes));
        } else {
            bottom_boxes.push(box_node.clone());
            real_node_guard.set_property(InternalProperties::BOTTOM_COMMENTS, Some(bottom_boxes));
        }
    }

    if let Ok(mut box_guard) = box_node.lock() {
        box_guard.set_property(
            InternalProperties::COMMENT_CONN_PORT,
            Some(opposite_port.clone()),
        );
    }

    let edge_targets_opposite = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.target())
        .map(|target| Arc::ptr_eq(&target, opposite_port))
        .unwrap_or(false);

    if edge_targets_opposite {
        LEdge::set_target(edge, None);
        if opposite_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.degree() == 0)
            .unwrap_or(false)
        {
            crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(opposite_port, None);
        }
        remove_hierarchical_port_dummy_node(opposite_port);
    } else {
        LEdge::set_source(edge, None);
        if opposite_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.degree() == 0)
            .unwrap_or(false)
        {
            crate::org::eclipse::elk::alg::layered::graph::LPort::set_node(opposite_port, None);
        }
    }

    if let Ok(mut edge_guard) = edge.lock() {
        edge_guard.bend_points().clear();
    }
}

fn choose_comment_side(real_node: &LNodeRef) -> (bool, bool, bool) {
    let (port_constraints, ports, labels, node_height) =
        if let Ok(mut real_node_guard) = real_node.lock() {
            (
                if real_node_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::PORT_CONSTRAINTS)
                {
                    real_node_guard
                        .get_property(LayeredOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined)
                } else {
                    PortConstraints::Undefined
                },
                real_node_guard.ports().clone(),
                real_node_guard.labels().clone(),
                real_node_guard.shape().size_ref().y,
            )
        } else {
            return (false, false, true);
        };

    let mut only_top = false;
    let mut only_bottom = false;
    if port_constraints.is_side_fixed() {
        let mut has_north = false;
        let mut has_south = false;
        'port_loop: for port in ports {
            let (side, connected_ports) = if let Ok(port_guard) = port.lock() {
                (port_guard.side(), port_guard.connected_ports())
            } else {
                continue;
            };
            for connected in connected_ports {
                let connected_node = connected
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                if connected_node.as_ref().map(is_comment_box).unwrap_or(false) {
                    continue;
                }
                if side == PortSide::North {
                    has_north = true;
                    break 'port_loop;
                }
                if side == PortSide::South {
                    has_south = true;
                }
            }
        }
        only_top = has_south && !has_north;
        only_bottom = has_north && !has_south;
    }

    let top_first = if !only_top && !only_bottom && !labels.is_empty() {
        let mut label_pos_sum = 0.0;
        let mut count = 0;
        for label in labels {
            if let Ok(mut label_guard) = label.lock() {
                label_pos_sum +=
                    label_guard.shape().position_ref().y + label_guard.shape().size_ref().y / 2.0;
                count += 1;
            }
        }
        if count == 0 {
            !only_bottom
        } else {
            (label_pos_sum / count as f64) >= node_height / 2.0
        }
    } else {
        !only_bottom
    };

    (only_top, only_bottom, top_first)
}

fn remove_hierarchical_port_dummy_node(opposite_port: &LPortRef) {
    let dummy = opposite_port.lock().ok().and_then(|mut port_guard| {
        if port_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(InternalProperties::PORT_DUMMY)
        {
            port_guard.get_property(InternalProperties::PORT_DUMMY)
        } else {
            None
        }
    });
    let Some(dummy) = dummy else {
        return;
    };

    let layer = dummy
        .lock()
        .ok()
        .and_then(|dummy_guard| dummy_guard.layer());
    let Some(layer) = layer else {
        return;
    };

    LNode::set_layer(&dummy, None);
    let layer_is_empty = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().is_empty())
        .unwrap_or(false);
    if !layer_is_empty {
        return;
    }

    if let Some(graph) = layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.graph())
    {
        if let Ok(mut graph_guard) = graph.lock() {
            graph_guard
                .layers_mut()
                .retain(|candidate| !Arc::ptr_eq(candidate, &layer));
        }
    }
}
