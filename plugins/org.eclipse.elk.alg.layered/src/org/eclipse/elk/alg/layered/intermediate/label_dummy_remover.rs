use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LLabelRef, LNode, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::LongEdgeJoiner;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

pub struct LabelDummyRemover;

impl ILayoutProcessor<LGraph> for LabelDummyRemover {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Label dummy removal", 1.0);

        let edge_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_LABEL)
            .unwrap_or(2.0);
        let label_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_LABEL_LABEL)
            .unwrap_or(1.0);
        let layout_direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Right);
        let edge_routing = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Orthogonal);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let is_label_dummy = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type() == NodeType::Label)
                    .unwrap_or(false);
                if !is_label_dummy {
                    continue;
                }

                Self::place_labels_and_restore_edge(
                    &node,
                    edge_label_spacing,
                    label_label_spacing,
                    layout_direction,
                );

                LongEdgeJoiner::join_at(&node, edge_routing == EdgeRouting::Polyline);
                Self::cleanup_label_dummy_bends(&node, layout_direction);
                LNode::set_layer(&node, None);
            }
        }

        monitor.done();
    }
}

impl LabelDummyRemover {
    fn place_labels_and_restore_edge(
        node: &LNodeRef,
        edge_label_spacing: f64,
        label_label_spacing: f64,
        layout_direction: Direction,
    ) {
        let (origin_edge, represented_labels, mut curr_label_pos, node_size, labels_below_edge, inline_labels) = {
            let mut node_guard = match node.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let origin_edge = match node_guard.get_property(InternalProperties::ORIGIN) {
                Some(Origin::LEdge(edge)) => edge,
                _ => return,
            };

            let represented_labels = node_guard
                .get_property(InternalProperties::REPRESENTED_LABELS)
                .unwrap_or_default();
            if represented_labels.is_empty() {
                return;
            }

            let thickness = Self::edge_thickness(&origin_edge);

            let labels_below_edge = node_guard
                .get_property(InternalProperties::LABEL_SIDE)
                .unwrap_or(LabelSide::Unknown)
                == LabelSide::Below;

            let mut curr_label_pos = KVector::from_vector(node_guard.shape().position_ref());
            Self::maybe_override_label_position(&origin_edge, layout_direction, &mut curr_label_pos);
            if labels_below_edge {
                curr_label_pos.y += thickness + edge_label_spacing;
            }

            let node_size = KVector::from_vector(node_guard.shape().size_ref());
            let inline_labels = represented_labels
                .iter()
                .all(Self::label_inline_property);

            (
                origin_edge,
                represented_labels,
                curr_label_pos,
                node_size,
                labels_below_edge,
                inline_labels,
            )
        };

        let label_space = KVector::with_values(
            node_size.x,
            node_size.y
                + if inline_labels {
                    0.0
                } else {
                    let thickness = Self::edge_thickness(&origin_edge);
                    -thickness - edge_label_spacing
                },
        );

        if layout_direction.is_vertical() {
            Self::place_labels_for_vertical_layout(
                &represented_labels,
                &mut curr_label_pos,
                label_label_spacing,
                &label_space,
                labels_below_edge,
                layout_direction,
            );
        } else {
            Self::place_labels_for_horizontal_layout(
                &represented_labels,
                &mut curr_label_pos,
                label_label_spacing,
                &label_space,
            );
        }

        if let Ok(mut edge_guard) = origin_edge.lock() {
            edge_guard.labels_mut().extend(represented_labels);
        };
    }

    fn maybe_override_label_position(
        origin_edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
        layout_direction: Direction,
        label_pos: &mut KVector,
    ) {
        if !layout_direction.is_horizontal() {
            return;
        }
        let Ok(edge_guard) = origin_edge.try_lock() else {
            return;
        };
        let source_anchor = edge_guard
            .source()
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()));
        let Some(source_anchor) = source_anchor else {
            return;
        };
        let bends = edge_guard.bend_points_ref().to_array();
        if bends.len() >= 2 {
            let p0 = bends[0];
            let p1 = bends[1];
            if approx_eq(p0.y, source_anchor.y) && !approx_eq(p1.y, source_anchor.y) {
                label_pos.y = source_anchor.y;
            }
        }
    }

    fn cleanup_label_dummy_bends(node: &LNodeRef, layout_direction: Direction) {
        if !layout_direction.is_horizontal() {
            return;
        }
        let origin_edge = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN))
            .and_then(|origin| match origin {
                Origin::LEdge(edge) => Some(edge),
                _ => None,
            });
        let Some(origin_edge) = origin_edge else {
            return;
        };
        let has_labels = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::REPRESENTED_LABELS))
            .map(|labels| !labels.is_empty())
            .unwrap_or(false);
        if !has_labels {
            return;
        }

        let source_anchor = origin_edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.absolute_anchor()));
        let Some(source_anchor) = source_anchor else {
            return;
        };

        let mut edge_guard = match origin_edge.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let bends = edge_guard.bend_points_ref().to_array();
        if bends.len() < 3 {
            return;
        }
        let p0 = bends[0];
        let p1 = bends[1];
        let p2 = bends[2];
        let next_x_aligned = bends
            .get(3)
            .map(|p3| approx_eq(p2.x, p3.x))
            .unwrap_or(true);
        if approx_eq(p0.y, source_anchor.y)
            && approx_eq(p0.x, p1.x)
            && approx_eq(p1.y, p2.y)
            && !approx_eq(p1.y, source_anchor.y)
            && next_x_aligned
        {
            let mut new_bends = Vec::with_capacity(bends.len().saturating_sub(2));
            new_bends.push(KVector::with_values(p2.x, source_anchor.y));
            if bends.len() > 3 {
                new_bends.extend_from_slice(&bends[3..]);
            }
            edge_guard.bend_points().clear();
            edge_guard.bend_points().add_all(&new_bends);
        }
    }

    fn place_labels_for_horizontal_layout(
        labels: &[LLabelRef],
        label_pos: &mut KVector,
        label_spacing: f64,
        label_space: &KVector,
    ) {
        for label in labels {
            if let Ok(mut label_guard) = label.lock() {
                let label_size_x = label_guard.shape().size_ref().x;
                let label_size_y = label_guard.shape().size_ref().y;

                label_guard.shape().position().x = label_pos.x + (label_space.x - label_size_x) / 2.0;
                label_guard.shape().position().y = label_pos.y;

                label_pos.y += label_size_y + label_spacing;
            }
        }
    }

    fn place_labels_for_vertical_layout(
        labels: &[LLabelRef],
        label_pos: &mut KVector,
        label_spacing: f64,
        label_space: &KVector,
        left_aligned: bool,
        layout_direction: Direction,
    ) {
        let inline = labels.iter().all(|label| {
            Self::label_inline_property(label)
        });

        if layout_direction == Direction::Up {
            for label in labels.iter().rev() {
                Self::place_vertical_label(
                    label,
                    label_pos,
                    label_spacing,
                    label_space,
                    inline,
                    left_aligned,
                );
            }
        } else {
            for label in labels {
                Self::place_vertical_label(
                    label,
                    label_pos,
                    label_spacing,
                    label_space,
                    inline,
                    left_aligned,
                );
            }
        }
    }

    fn place_vertical_label(
        label: &LLabelRef,
        label_pos: &mut KVector,
        label_spacing: f64,
        label_space: &KVector,
        inline: bool,
        left_aligned: bool,
    ) {
        if let Ok(mut label_guard) = label.lock() {
            let label_size_x = label_guard.shape().size_ref().x;
            let label_size_y = label_guard.shape().size_ref().y;

            label_guard.shape().position().x = label_pos.x;
            if inline {
                label_guard.shape().position().y = label_pos.y + (label_space.y - label_size_y) / 2.0;
            } else if left_aligned {
                label_guard.shape().position().y = label_pos.y;
            } else {
                label_guard.shape().position().y = label_pos.y + label_space.y - label_size_y;
            }

            label_pos.x += label_size_x + label_spacing;
        }
    }

    fn edge_thickness(edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef) -> f64 {
        edge.lock()
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
            .unwrap_or(1.0)
    }

    fn label_inline_property(label: &LLabelRef) -> bool {
        label
            .lock()
            .ok()
            .and_then(|mut label_guard| {
                if label_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::EDGE_LABELS_INLINE)
                {
                    label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE)
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}
