use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LLabelRef, LNode, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::LongEdgeSplitter;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

pub struct LabelDummyInserter;

impl ILayoutProcessor<LGraph> for LabelDummyInserter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Label dummy insertions", 1.0);

        let edge_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_LABEL)
            .unwrap_or(2.0);
        let label_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_LABEL_LABEL)
            .unwrap_or(1.0);
        let layout_direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Right);

        let nodes = layered_graph.layerless_nodes().clone();
        let mut new_dummy_nodes = Vec::new();

        for node in nodes {
            let graph = node.lock().ok().and_then(|node_guard| node_guard.graph());
            let Some(graph) = graph else {
                continue;
            };

            let outgoing_edges = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing_edges {
                if !edge_needs_processing(&edge) {
                    continue;
                }

                let thickness = retrieve_thickness(&edge);
                let dummy_node = create_label_dummy(&graph, &edge, thickness);
                new_dummy_nodes.push(dummy_node.clone());

                let (represented_labels, dummy_size) =
                    collect_center_labels_and_size(
                        &edge,
                        layout_direction,
                        thickness,
                        edge_label_spacing,
                        label_label_spacing,
                    );

                if let Ok(mut dummy_guard) = dummy_node.lock() {
                    let size = dummy_guard.shape().size();
                    size.x = dummy_size.x;
                    size.y = dummy_size.y;
                    dummy_guard.set_property(
                        InternalProperties::REPRESENTED_LABELS,
                        Some(represented_labels),
                    );
                };
            }
        }

        layered_graph.layerless_nodes_mut().extend(new_dummy_nodes);
        monitor.done();
    }
}

fn edge_needs_processing(edge: &LEdgeRef) -> bool {
    let (is_self_loop, labels) = edge
        .lock()
        .ok()
        .map(|edge_guard| (edge_guard.is_self_loop(), edge_guard.labels().clone()))
        .unwrap_or((true, Vec::new()));
    if is_self_loop {
        return false;
    }
    labels.iter().any(|label| label_is_center(label))
}

fn label_is_center(label: &LLabelRef) -> bool {
    label
        .lock()
        .ok()
        .and_then(|mut label_guard| {
            label_guard.get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
        })
        .unwrap_or(EdgeLabelPlacement::Center)
        == EdgeLabelPlacement::Center
}

fn retrieve_thickness(edge: &LEdgeRef) -> f64 {
    let thickness = edge
        .lock()
        .ok()
        .and_then(|mut edge_guard| {
            if edge_guard
                .graph_element()
                .properties()
                .has_property(CoreOptions::EDGE_THICKNESS)
            {
                edge_guard.get_property(CoreOptions::EDGE_THICKNESS)
            } else {
                None
            }
        })
        .unwrap_or(1.0);

    if thickness < 0.0 {
        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(0.0));
        }
        0.0
    } else {
        thickness
    }
}

fn create_label_dummy(graph: &crate::org::eclipse::elk::alg::layered::graph::LGraphRef,
    edge: &LEdgeRef,
    thickness: f64,
) -> LNodeRef {
    let dummy_node = LNode::new(graph);
    let source = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
    let target = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
    if let Ok(mut dummy_guard) = dummy_node.lock() {
        dummy_guard.set_node_type(NodeType::Label);
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(edge.clone())),
        );
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        if let Some(source) = source {
            dummy_guard.set_property(InternalProperties::LONG_EDGE_SOURCE, Some(source));
        }
        if let Some(target) = target {
            dummy_guard.set_property(InternalProperties::LONG_EDGE_TARGET, Some(target));
        }
    }

    LongEdgeSplitter::split_edge(edge, &dummy_node);

    let port_pos = (thickness / 2.0).floor();
    if let Ok(dummy_guard) = dummy_node.lock() {
        let ports = dummy_guard.ports().clone();
        for port in ports {
            if let Ok(mut port_guard) = port.lock() {
                port_guard.shape().position().y = port_pos;
            }
        }
    }

    dummy_node
}

fn collect_center_labels_and_size(
    edge: &LEdgeRef,
    layout_direction: Direction,
    thickness: f64,
    edge_label_spacing: f64,
    label_label_spacing: f64,
) -> (Vec<LLabelRef>, KVector) {
    let labels = edge
        .lock()
        .ok()
        .map(|edge_guard| edge_guard.labels().clone())
        .unwrap_or_default();

    let mut represented_labels = Vec::new();
    let mut remaining_labels = Vec::new();
    let mut dummy_size = KVector::with_values(0.0, thickness);

    for label in labels {
        let (placement, label_size) = match label.lock() {
            Ok(mut label_guard) => {
                let placement = label_guard
                    .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                    .unwrap_or(EdgeLabelPlacement::Center);
                let size = *label_guard.shape().size_ref();
                (placement, size)
            }
            Err(_) => {
                remaining_labels.push(label.clone());
                continue;
            }
        };

        if placement == EdgeLabelPlacement::Center {
            if layout_direction.is_vertical() {
                dummy_size.x += label_size.x + label_label_spacing;
                dummy_size.y = dummy_size.y.max(label_size.y);
            } else {
                dummy_size.x = dummy_size.x.max(label_size.x);
                dummy_size.y += label_size.y + label_label_spacing;
            }
            represented_labels.push(label);
        } else {
            remaining_labels.push(label);
        }
    }

    if !represented_labels.is_empty() {
        if layout_direction.is_vertical() {
            dummy_size.x -= label_label_spacing;
            dummy_size.y += edge_label_spacing + thickness;
        } else {
            dummy_size.y += edge_label_spacing - label_label_spacing + thickness;
        }
    }

    if let Ok(mut edge_guard) = edge.lock() {
        *edge_guard.labels_mut() = remaining_labels;
    }

    (represented_labels, dummy_size)
}
