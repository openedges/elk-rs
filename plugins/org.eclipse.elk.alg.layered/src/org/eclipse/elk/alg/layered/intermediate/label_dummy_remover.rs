use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LLabelRef, LNode, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::LongEdgeJoiner;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphCompactionStrategy, InternalProperties, LayeredOptions, Origin, Spacings,
};

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
        let compaction_strategy = layered_graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY)
            .unwrap_or(GraphCompactionStrategy::None);
        let spacings = layered_graph.get_property(InternalProperties::SPACINGS);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock().nodes().clone();

            for node in nodes {
                let is_label_dummy = node.lock().node_type() == NodeType::Label;
                if !is_label_dummy {
                    continue;
                }

                Self::place_labels_and_restore_edge(
                    &node,
                    edge_label_spacing,
                    label_label_spacing,
                    layout_direction,
                    edge_routing,
                    compaction_strategy,
                    spacings.as_ref(),
                );

                LongEdgeJoiner::join_at(&node, edge_routing == EdgeRouting::Polyline);
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
        edge_routing: EdgeRouting,
        compaction_strategy: GraphCompactionStrategy,
        spacings: Option<&Spacings>,
    ) {
        let (
            origin_edge,
            represented_labels,
            mut curr_label_pos,
            node_size,
            labels_below_edge,
            inline_labels,
        ) = {
            let mut node_guard = node.lock();

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
            if labels_below_edge {
                curr_label_pos.y += thickness + edge_label_spacing;
            }

            let node_size = KVector::from_vector(node_guard.shape().size_ref());
            let inline_labels = represented_labels.iter().all(Self::label_inline_property);

            (
                origin_edge,
                represented_labels,
                curr_label_pos,
                node_size,
                labels_below_edge,
                inline_labels,
            )
        };

        if !layout_direction.is_vertical() && edge_routing != EdgeRouting::Splines {
            Self::adjust_horizontal_dummy_position_for_post_compaction(
                node,
                &mut curr_label_pos,
                node_size.x,
                compaction_strategy,
                spacings,
            );
        }

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

        {
            let mut edge_guard = origin_edge.lock();
            edge_guard.labels_mut().extend(represented_labels);
        };
    }

    fn adjust_horizontal_dummy_position_for_post_compaction(
        node: &LNodeRef,
        curr_label_pos: &mut KVector,
        node_width: f64,
        compaction_strategy: GraphCompactionStrategy,
        spacings: Option<&Spacings>,
    ) {
        if compaction_strategy != GraphCompactionStrategy::Left
            && compaction_strategy != GraphCompactionStrategy::Right
        {
            return;
        }
        let Some(spacings) = spacings else {
            return;
        };
        let Some((source_node, target_node)) = Self::adjacent_source_and_target_nodes(node) else {
            return;
        };

        let source_right = {
            let mut source_guard = source_node.lock();
            source_guard.shape().position_ref().x + source_guard.shape().size_ref().x
        };
        let target_left = target_node.lock().shape().position_ref().x;

        let left_spacing = spacings.get_horizontal_spacing(&source_node, node);
        let right_spacing = spacings.get_horizontal_spacing(node, &target_node);
        let left_bound = source_right + left_spacing;
        let right_bound = target_left - right_spacing - node_width;
        if !left_bound.is_finite() || !right_bound.is_finite() {
            return;
        }

        if left_bound > right_bound {
            curr_label_pos.x = left_bound;
            return;
        }
        curr_label_pos.x = if compaction_strategy == GraphCompactionStrategy::Left {
            left_bound
        } else {
            right_bound
        };
    }

    fn adjacent_source_and_target_nodes(node: &LNodeRef) -> Option<(LNodeRef, LNodeRef)> {
        let (west_port, east_port) = {
            let node_guard = node.lock();
            (
                node_guard.ports_by_side(PortSide::West).first().cloned(),
                node_guard.ports_by_side(PortSide::East).first().cloned(),
            )
        };
        let incoming_edge = west_port
            .and_then(|port| port.lock().incoming_edges().first().cloned())?;
        let outgoing_edge = east_port
            .and_then(|port| port.lock().outgoing_edges().first().cloned())?;

        let source_node = incoming_edge
            .lock().source()
            .and_then(|port| port.lock().node())?;
        let target_node = outgoing_edge
            .lock().target()
            .and_then(|port| port.lock().node())?;

        Some((source_node, target_node))
    }

    fn place_labels_for_horizontal_layout(
        labels: &[LLabelRef],
        label_pos: &mut KVector,
        label_spacing: f64,
        label_space: &KVector,
    ) {
        for label in labels {
            {
                let mut label_guard = label.lock();
                let label_size_x = label_guard.shape().size_ref().x;
                let label_size_y = label_guard.shape().size_ref().y;

                label_guard.shape().position().x =
                    label_pos.x + (label_space.x - label_size_x) / 2.0;
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
        let inline = labels
            .iter()
            .all(Self::label_inline_property);

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
        {
            let mut label_guard = label.lock();
            let label_size_x = label_guard.shape().size_ref().x;
            let label_size_y = label_guard.shape().size_ref().y;

            label_guard.shape().position().x = label_pos.x;
            if inline {
                label_guard.shape().position().y =
                    label_pos.y + (label_space.y - label_size_y) / 2.0;
            } else if left_aligned {
                label_guard.shape().position().y = label_pos.y;
            } else {
                label_guard.shape().position().y = label_pos.y + label_space.y - label_size_y;
            }

            label_pos.x += label_size_x + label_spacing;
        }
    }

    fn edge_thickness(edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef) -> f64 {
        let mut edge_guard = edge.lock();
        if edge_guard
            .graph_element()
            .properties()
            .has_property(CoreOptions::EDGE_THICKNESS)
        {
            edge_guard.get_property(CoreOptions::EDGE_THICKNESS).unwrap_or(1.0)
        } else {
            1.0
        }
    }

    fn label_inline_property(label: &LLabelRef) -> bool {
        let mut label_guard = label.lock();
        if label_guard
            .shape()
            .graph_element()
            .properties()
            .has_property(LayeredOptions::EDGE_LABELS_INLINE)
        {
            label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE).unwrap_or(false)
        } else {
            false
        }
    }
}
