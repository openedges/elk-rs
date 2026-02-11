use std::any::Any;
use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, PortConstraints, PortLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapter, ElkLabelAdapter, ElkNodeAdapter, ElkPortAdapter, GraphAdapter,
    GraphElementAdapter, NodeAdapter, PortAdapter,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef, ElkPortRef};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use std::rc::Rc;

use super::node_label_and_size_calculator::NodeLabelAndSizeCalculator;
use super::node_margin_calculator::NodeMarginCalculator;

pub struct NodeDimensionCalculation;

#[derive(Clone, Copy)]
struct HorizontalStackConfig {
    positive_direction: bool,
    gap_x: f64,
    gap_y: f64,
    start_coordinate: Option<f64>,
    clamp_x: Option<(f64, f64)>,
}

impl NodeDimensionCalculation {
    pub fn calculate_label_and_node_sizes<T, G>(adapter: &G)
    where
        T: 'static,
        G: GraphAdapter<T> + Any,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Graph: GraphElementAdapter<<G as GraphAdapter<T>>::Node>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Label: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::LabelAdapter: 'static,
    {
        // Java: single path — NodeLabelAndSizeCalculator.process(adapter)
        // which iterates nodes and calls process_node() for each.
        // We match this by calling process_node for each node, then handling port labels.
        //
        // For ELK adapters we use the ELK-specific path that has full endpoint analysis.
        // For other adapters (e.g. LGraphAdapter), we use process_node + generic port label path.
        if let Some(elk_adapter) = (adapter as &dyn Any).downcast_ref::<ElkGraphAdapter>() {
            Self::calculate_label_and_node_sizes_for_elk(elk_adapter);
            return;
        }

        // Note: _with_process_node and _generic produce identical results because
        // LNodeAdapter::get_graph() returns None, making process_node skip its logic.
        // Phase 1 port placement in LabelAndNodeSizeProcessor handles what process_node
        // would do in the Java ELK path. Keep _generic for now.
        Self::calculate_label_and_node_sizes_generic(adapter);
    }

    pub fn calculate_label_and_node_sizes_for_elk(adapter: &ElkGraphAdapter) {
        let layout_direction = adapter
            .get_property(CoreOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        for node in adapter.get_nodes() {
            NodeLabelAndSizeCalculator::process_node(&node, layout_direction);

            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let constrained_placement = Self::has_constrained_port_label_placement(&node);
            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side = placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;

            let ports = node.get_ports();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            });
            // Compute per-side port counts for label_placement_relation
            let mut side_counts: [usize; 5] = [0; 5];
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];
            let mut north_entries = Vec::new();
            let mut south_entries = Vec::new();

            for port in ports.iter() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index = if port.get_side() == PortSide::West || port.get_side() == PortSide::South {
                    per_side_count.saturating_sub(1).saturating_sub(per_side_index)
                } else {
                    per_side_index
                };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port(port, &node, inside_label_placement),
                );

                let port_size = port.get_size();
                let port_border_offset =
                    port.get_property(CoreOptions::PORT_BORDER_OFFSET).unwrap_or(0.0);
                let label_border_offset = Self::port_label_border_offset_for_port_side(
                    &node,
                    port.get_side(),
                    label_gap_horizontal,
                    label_gap_vertical,
                );
                let labels = port.get_labels();
                let label_count = labels.len();
                let total_label_height: f64 = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };
                let mut y_cursor = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    if inside_label_placement {
                        (port_size.y - total_label_height) / 2.0
                    } else {
                        match relation {
                            LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                            LabelPlacementRelation::AboveOrLeft => -total_label_height - 1.0,
                            LabelPlacementRelation::Centered => (port_size.y - total_label_height) / 2.0,
                        }
                    }
                } else {
                    0.0
                };
                for label in labels {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + port_border_offset + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + port_border_offset + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                    }
                    label.set_position(label_pos);

                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port.clone(), label.clone())),
                            PortSide::South => south_entries.push((port.clone(), label.clone())),
                            _ => {}
                        }
                    }
                }
            }

            if constrained_placement {
                let north_positive = inside_label_placement;
                let south_positive = !inside_label_placement;
                let clamp_x = inside_label_placement
                    .then_some(Self::inside_horizontal_label_clamp_bounds(&node, node_width));

                // Java uses a strip overlap remover with side-dependent start coordinates.
                // This simplified variant keeps the same directional stacking behavior.
                Self::stack_horizontal_side_labels(
                    &north_entries,
                    HorizontalStackConfig {
                        positive_direction: north_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate(
                            &north_entries,
                            north_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
                Self::stack_horizontal_side_labels(
                    &south_entries,
                    HorizontalStackConfig {
                        positive_direction: south_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate(
                            &south_entries,
                            south_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
            }
        }
    }

    /// Java-matching path: calls process_node (for node label sizing) then port label placement.
    /// This is what Java's NodeDimensionCalculation.calculateLabelAndNodeSizes does via
    /// NodeLabelAndSizeCalculator.process(adapter).
    fn calculate_label_and_node_sizes_with_process_node<T, G>(adapter: &G)
    where
        T: 'static,
        G: GraphAdapter<T>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Graph: GraphElementAdapter<<G as GraphAdapter<T>>::Node>,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::Label: 'static,
        <<G as GraphAdapter<T>>::NodeAdapter as NodeAdapter<<G as GraphAdapter<T>>::Node>>::LabelAdapter: 'static,
        <G as GraphAdapter<T>>::Node: 'static,
    {
        let layout_direction = adapter
            .get_property(CoreOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        for node in adapter.get_nodes() {
            NodeLabelAndSizeCalculator::process_node(&node, layout_direction);

            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let constrained_placement = placement.contains(&PortLabelPlacement::Inside)
                || placement.contains(&PortLabelPlacement::Outside);
            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side = placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;

            let ports = node.get_ports();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            });
            let mut side_counts: [usize; 5] = [0; 5];
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];
            let mut north_entries: Vec<(usize, usize)> = Vec::new();
            let mut south_entries: Vec<(usize, usize)> = Vec::new();

            for (port_idx, port) in ports.iter().enumerate() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index = if port.get_side() == PortSide::West || port.get_side() == PortSide::South {
                    per_side_count.saturating_sub(1).saturating_sub(per_side_index)
                } else {
                    per_side_index
                };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port_generic(port, inside_label_placement),
                );

                let port_size = port.get_size();
                let port_border_offset =
                    port.get_property(CoreOptions::PORT_BORDER_OFFSET).unwrap_or(0.0);
                let label_border_offset = {
                    let padding = node.get_padding();
                    match port.get_side() {
                        PortSide::North => padding.top + label_gap_vertical,
                        PortSide::East => padding.right + label_gap_horizontal,
                        PortSide::South => padding.bottom + label_gap_vertical,
                        PortSide::West | PortSide::Undefined => padding.left + label_gap_horizontal,
                    }
                };
                let labels = port.get_labels();
                let label_count = labels.len();
                let total_label_height: f64 = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };

                let mut y_cursor = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    if inside_label_placement {
                        (port_size.y - total_label_height) / 2.0
                    } else {
                        match relation {
                            LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                            LabelPlacementRelation::AboveOrLeft => -total_label_height - 1.0,
                            LabelPlacementRelation::Centered => (port_size.y - total_label_height) / 2.0,
                        }
                    }
                } else {
                    0.0
                };

                for (label_idx, label) in labels.iter().enumerate() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + port_border_offset + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + port_border_offset + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                    }
                    label.set_position(label_pos);

                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port_idx, label_idx)),
                            PortSide::South => south_entries.push((port_idx, label_idx)),
                            _ => {}
                        }
                    }
                }
            }

            if constrained_placement {
                let north_positive = inside_label_placement;
                let south_positive = !inside_label_placement;
                let clamp_x = inside_label_placement
                    .then(|| Self::inside_horizontal_label_clamp_bounds_generic(&node, node_width));

                Self::stack_horizontal_side_labels_generic(
                    &node,
                    &north_entries,
                    HorizontalStackConfig {
                        positive_direction: north_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate_generic(
                            &node,
                            &north_entries,
                            north_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
                Self::stack_horizontal_side_labels_generic(
                    &node,
                    &south_entries,
                    HorizontalStackConfig {
                        positive_direction: south_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate_generic(
                            &node,
                            &south_entries,
                            south_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
            }
        }
    }

    fn calculate_label_and_node_sizes_generic<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        for node in adapter.get_nodes() {
            let placement = node
                .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                .unwrap_or_default();
            if PortLabelPlacement::is_fixed(&placement) {
                continue;
            }

            let inside_label_placement = placement.contains(&PortLabelPlacement::Inside);
            let next_to_port_if_possible =
                placement.contains(&PortLabelPlacement::NextToPortIfPossible);
            let always_same_side = placement.contains(&PortLabelPlacement::AlwaysSameSide);
            let always_other_same_side = placement.contains(&PortLabelPlacement::AlwaysOtherSameSide);
            let space_efficient = placement.contains(&PortLabelPlacement::SpaceEfficient);

            let ports = node.get_ports();
            let any_incident_edges = ports.iter().any(|port| {
                !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty()
            });

            // Compute per-side port counts (Java uses per-side index/count for label_placement_relation)
            let mut side_counts: [usize; 5] = [0; 5]; // N=0, E=1, S=2, W=3, Undef=4
            for port in ports.iter() {
                let idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                side_counts[idx] += 1;
            }
            let mut side_indices: [usize; 5] = [0; 5];

            let constrained_placement = placement.contains(&PortLabelPlacement::Inside)
                || placement.contains(&PortLabelPlacement::Outside);
            let label_gap_horizontal = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_HORIZONTAL)
                .unwrap_or(1.0);
            let label_gap_vertical = node
                .get_property(CoreOptions::SPACING_LABEL_PORT_VERTICAL)
                .unwrap_or(1.0);
            let node_width = node.get_size().x;

            // Collect north/south entries for stacking (generic version stores tuples)
            let mut north_entries: Vec<(usize, usize)> = Vec::new(); // (port_idx, label_idx)
            let mut south_entries: Vec<(usize, usize)> = Vec::new();

            for (port_idx, port) in ports.iter().enumerate() {
                let side_idx = match port.get_side() {
                    PortSide::North => 0,
                    PortSide::East => 1,
                    PortSide::South => 2,
                    PortSide::West => 3,
                    PortSide::Undefined => 4,
                };
                let per_side_index = side_indices[side_idx];
                let per_side_count = side_counts[side_idx];
                side_indices[side_idx] += 1;

                // Java's NodeContext.comparePortContexts reverses WEST/SOUTH ports in TreeMultimap
                let effective_index = if port.get_side() == PortSide::West || port.get_side() == PortSide::South {
                    per_side_count.saturating_sub(1).saturating_sub(per_side_index)
                } else {
                    per_side_index
                };

                let relation = Self::label_placement_relation(
                    effective_index,
                    per_side_count,
                    any_incident_edges,
                    inside_label_placement,
                    next_to_port_if_possible,
                    always_same_side,
                    always_other_same_side,
                    space_efficient,
                    Self::should_label_be_placed_next_to_port_generic(port, inside_label_placement),
                );

                let port_size = port.get_size();
                let port_border_offset =
                    port.get_property(CoreOptions::PORT_BORDER_OFFSET).unwrap_or(0.0);
                let label_border_offset = {
                    let padding = node.get_padding();
                    match port.get_side() {
                        PortSide::North => padding.top + label_gap_vertical,
                        PortSide::East => padding.right + label_gap_horizontal,
                        PortSide::South => padding.bottom + label_gap_vertical,
                        PortSide::West | PortSide::Undefined => padding.left + label_gap_horizontal,
                    }
                };
                let labels = port.get_labels();
                let label_count = labels.len();
                // For EAST/WEST stacking: compute total label height for centering
                let total_label_height: f64 = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    labels.iter().map(|l| l.get_size().y).sum()
                } else {
                    0.0
                };

                let mut y_cursor = if label_count > 1
                    && matches!(port.get_side(), PortSide::East | PortSide::West | PortSide::Undefined)
                {
                    if inside_label_placement {
                        (port_size.y - total_label_height) / 2.0
                    } else {
                        match relation {
                            LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                            LabelPlacementRelation::AboveOrLeft => -total_label_height - 1.0,
                            LabelPlacementRelation::Centered => (port_size.y - total_label_height) / 2.0,
                        }
                    }
                } else {
                    0.0
                };

                for (label_idx, label) in labels.iter().enumerate() {
                    let mut label_pos = label.get_position();
                    let label_size = label.get_size();
                    match port.get_side() {
                        PortSide::North => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    port_size.y + port_border_offset + label_border_offset
                                } else {
                                    -label_size.y - label_gap_vertical
                                };
                            }
                        }
                        PortSide::South => {
                            label_pos.x = match relation {
                                LabelPlacementRelation::Centered => {
                                    (port_size.x - label_size.x) / 2.0
                                }
                                LabelPlacementRelation::BelowOrRight => port_size.x + 1.0,
                                LabelPlacementRelation::AboveOrLeft => -label_size.x - 1.0,
                            };
                            if constrained_placement {
                                label_pos.y = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.y
                                } else {
                                    port_size.y + label_gap_vertical
                                };
                            }
                        }
                        PortSide::East => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    -port_border_offset - label_border_offset - label_size.x
                                } else {
                                    port_size.x + label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                        PortSide::West | PortSide::Undefined => {
                            if constrained_placement {
                                label_pos.x = if inside_label_placement {
                                    port_size.x + port_border_offset + label_border_offset
                                } else {
                                    -label_size.x - label_gap_horizontal
                                };
                            }
                            if label_count > 1 {
                                label_pos.y = y_cursor;
                                y_cursor += label_size.y;
                            } else {
                                label_pos.y = match relation {
                                    LabelPlacementRelation::Centered => {
                                        (port_size.y - label_size.y) / 2.0
                                    }
                                    LabelPlacementRelation::BelowOrRight => port_size.y + 1.0,
                                    LabelPlacementRelation::AboveOrLeft => -label_size.y - 1.0,
                                };
                            }
                        }
                    }
                    label.set_position(label_pos);

                    // Collect north/south entries for stacking
                    if constrained_placement {
                        match port.get_side() {
                            PortSide::North => north_entries.push((port_idx, label_idx)),
                            PortSide::South => south_entries.push((port_idx, label_idx)),
                            _ => {}
                        }
                    }
                }
            }

            // Apply stacking to north/south labels (generic version using stored indices)
            if constrained_placement {
                let north_positive = inside_label_placement;
                let south_positive = !inside_label_placement;
                let clamp_x = inside_label_placement
                    .then(|| Self::inside_horizontal_label_clamp_bounds_generic(&node, node_width));

                Self::stack_horizontal_side_labels_generic(
                    &node,
                    &north_entries,
                    HorizontalStackConfig {
                        positive_direction: north_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate_generic(
                            &node,
                            &north_entries,
                            north_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
                Self::stack_horizontal_side_labels_generic(
                    &node,
                    &south_entries,
                    HorizontalStackConfig {
                        positive_direction: south_positive,
                        gap_x: label_gap_horizontal,
                        gap_y: label_gap_vertical,
                        start_coordinate: Self::compute_stack_start_coordinate_generic(
                            &node,
                            &south_entries,
                            south_positive,
                            label_gap_vertical,
                        ),
                        clamp_x,
                    },
                );
            }
        }
    }

    pub fn calculate_node_margins<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        let mut calculator = NodeMarginCalculator::new(adapter);
        calculator.process();
    }

    pub fn get_node_margin_calculator<T, G>(adapter: &G) -> NodeMarginCalculator<'_, G, T>
    where
        G: GraphAdapter<T>,
    {
        NodeMarginCalculator::new(adapter)
    }

    pub fn sort_port_lists<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        for node in adapter.get_nodes() {
            node.sort_port_list();
        }
    }

    fn should_label_be_placed_next_to_port(
        port: &ElkPortAdapter,
        parent: &ElkNodeAdapter,
        inside_label_placement: bool,
    ) -> bool {
        let parent_node = parent.element();
        let port_node = port.element();
        let mut edges_to_insides = false;
        let mut edges_to_somewhere_else = false;
        let mut found_incident_edge = false;

        let incoming_edges = port.get_incoming_edges();
        let outgoing_edges = port.get_outgoing_edges();

        for out_edge in outgoing_edges {
            found_incident_edge = true;
            let target = {
                let edge_ref = out_edge.element();
                let edge = edge_ref.borrow();
                edge.targets_ro().get(0)
            };
            if let Some(target_shape) = target {
                Self::classify_endpoint_against_parent(
                    &target_shape,
                    &parent_node,
                    &mut edges_to_insides,
                    &mut edges_to_somewhere_else,
                );
            }
        }

        for in_edge in incoming_edges {
            found_incident_edge = true;
            let source = {
                let edge_ref = in_edge.element();
                let edge = edge_ref.borrow();
                edge.sources_ro().get(0)
            };
            if let Some(source_shape) = source {
                Self::classify_endpoint_against_parent(
                    &source_shape,
                    &parent_node,
                    &mut edges_to_insides,
                    &mut edges_to_somewhere_else,
                );
            }
        }

        if !found_incident_edge {
            found_incident_edge = Self::classify_incident_edges_from_containment(
                &port_node,
                &parent_node,
                &mut edges_to_insides,
                &mut edges_to_somewhere_else,
            );
        }

        if !found_incident_edge {
            return true;
        }

        (inside_label_placement && !edges_to_insides)
            || (!inside_label_placement && !edges_to_somewhere_else)
    }

    fn classify_endpoint_against_parent(
        endpoint: &ElkConnectableShapeRef,
        parent_node: &ElkNodeRef,
        edges_to_insides: &mut bool,
        edges_to_somewhere_else: &mut bool,
    ) {
        if let Some(endpoint_node) = ElkGraphUtil::connectable_shape_to_node(endpoint) {
            let inside_edge = Rc::ptr_eq(&endpoint_node, parent_node)
                || ElkGraphUtil::is_descendant(&endpoint_node, parent_node);
            *edges_to_insides |= inside_edge;
            *edges_to_somewhere_else |= !inside_edge;
        }
    }

    fn classify_incident_edges_from_containment(
        port: &ElkPortRef,
        parent_node: &ElkNodeRef,
        edges_to_insides: &mut bool,
        edges_to_somewhere_else: &mut bool,
    ) -> bool {
        let contained_edges: Vec<ElkEdgeRef> = parent_node
            .borrow_mut()
            .contained_edges()
            .iter()
            .cloned()
            .collect();

        let mut found_incident_edge = false;
        for edge_ref in contained_edges {
            let edge = edge_ref.borrow();
            let source_matches = edge.sources_ro().iter().any(|source| {
                matches!(source, ElkConnectableShapeRef::Port(source_port) if Rc::ptr_eq(source_port, port))
            });
            if source_matches {
                found_incident_edge = true;
                for target in edge.targets_ro().iter() {
                    Self::classify_endpoint_against_parent(
                        target,
                        parent_node,
                        edges_to_insides,
                        edges_to_somewhere_else,
                    );
                }
            }

            let target_matches = edge.targets_ro().iter().any(|target| {
                matches!(target, ElkConnectableShapeRef::Port(target_port) if Rc::ptr_eq(target_port, port))
            });
            if target_matches {
                found_incident_edge = true;
                for source in edge.sources_ro().iter() {
                    Self::classify_endpoint_against_parent(
                        source,
                        parent_node,
                        edges_to_insides,
                        edges_to_somewhere_else,
                    );
                }
            }
        }

        found_incident_edge
    }

    fn should_label_be_placed_next_to_port_generic<T, P>(
        port: &P,
        inside_label_placement: bool,
    ) -> bool
    where
        P: PortAdapter<T>,
    {
        let has_incident_edge =
            !port.get_incoming_edges().is_empty() || !port.get_outgoing_edges().is_empty();
        if !has_incident_edge {
            return true;
        }

        // Generic adapters do not expose endpoint ownership, so use a conservative fallback.
        !inside_label_placement
    }

    #[allow(clippy::too_many_arguments)]
    fn label_placement_relation(
        index: usize,
        port_count: usize,
        any_incident_edges: bool,
        inside_label_placement: bool,
        next_to_port_if_possible: bool,
        always_same_side: bool,
        always_other_same_side: bool,
        space_efficient: bool,
        should_place_next_to_port: bool,
    ) -> LabelPlacementRelation {
        let labels_next_to_port = next_to_port_if_possible && should_place_next_to_port;

        if next_to_port_if_possible {
            if labels_next_to_port {
                LabelPlacementRelation::Centered
            } else {
                LabelPlacementRelation::BelowOrRight
            }
        } else if inside_label_placement {
            if any_incident_edges {
                LabelPlacementRelation::BelowOrRight
            } else {
                LabelPlacementRelation::Centered
            }
        } else if always_same_side {
            LabelPlacementRelation::BelowOrRight
        } else if always_other_same_side
            || (!labels_next_to_port && (port_count == 2 || space_efficient) && index == 0)
        {
            LabelPlacementRelation::AboveOrLeft
        } else {
            LabelPlacementRelation::BelowOrRight
        }
    }

    fn has_constrained_port_label_placement(node: &ElkNodeAdapter) -> bool {
        let size_constraints = node
            .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
            .unwrap_or_default();
        let port_constraints = node
            .get_property(CoreOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);

        !size_constraints.contains(&SizeConstraint::PortLabels)
            || port_constraints == PortConstraints::FixedPos
    }

    fn port_label_border_offset_for_port_side(
        node: &ElkNodeAdapter,
        side: PortSide,
        horizontal_spacing: f64,
        vertical_spacing: f64,
    ) -> f64 {
        let padding = node.get_padding();
        match side {
            PortSide::North => padding.top + vertical_spacing,
            PortSide::East => padding.right + horizontal_spacing,
            PortSide::South => padding.bottom + vertical_spacing,
            PortSide::West | PortSide::Undefined => padding.left + horizontal_spacing,
        }
    }

    fn inside_horizontal_label_clamp_bounds(node: &ElkNodeAdapter, node_width: f64) -> (f64, f64) {
        let width = node_width.max(0.0);
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = width;

        // Keep defaults stable; only explicit properties influence clamp bounds.
        if node.has_property(CoreOptions::PADDING) {
            let padding = node.get_padding();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }
        if node.has_property(CoreOptions::SPACING_LABEL_NODE) {
            let spacing = node
                .get_property(CoreOptions::SPACING_LABEL_NODE)
                .unwrap_or(0.0)
                .max(0.0);
            min_x = min_x.max(spacing);
            max_x = (width - spacing).max(min_x);
        }
        if node.has_property(CoreOptions::NODE_LABELS_PADDING) {
            let padding = node
                .get_property(CoreOptions::NODE_LABELS_PADDING)
                .unwrap_or_default();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }

        (min_x, max_x)
    }

    fn stack_horizontal_side_labels(
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
        config: HorizontalStackConfig,
    ) {
        if entries.is_empty() {
            return;
        }

        // Java constrainedOutside falls back to simple placement for <=2 ports.
        // Keep that behavior, but allow constrainedInside clamping for small counts.
        if entries.len() <= 2 && config.clamp_x.is_none() {
            return;
        }

        let mut sorted = entries.to_vec();
        sorted.sort_by(|(left_port, left_label), (right_port, right_label)| {
            let left_x = left_port.get_position().x + left_label.get_position().x;
            let right_x = right_port.get_position().x + right_label.get_position().x;
            left_x.partial_cmp(&right_x).unwrap_or(Ordering::Equal)
        });

        let mut placed_rectangles: Vec<(f64, f64, f64, f64)> = Vec::new();
        for (port, label) in sorted {
            let port_position = port.get_position();
            let port_size = port.get_size();
            let label_size = label.get_size();
            let mut label_position = label.get_position();

            let mut absolute_x = port_position.x + label_position.x;
            if let Some((min_x, max_x)) = config.clamp_x {
                // Keep labels from drifting past the port extent at node boundaries.
                let actual_min_x = min_x.min(port_position.x);
                let actual_max_x = max_x.max(port_position.x + port_size.x);
                if absolute_x < actual_min_x {
                    absolute_x = actual_min_x;
                } else if absolute_x + label_size.x > actual_max_x {
                    absolute_x = actual_max_x - label_size.x;
                }
            }
            let mut absolute_y = match config.start_coordinate {
                Some(start) => {
                    if config.positive_direction {
                        start
                    } else {
                        start - label_size.y
                    }
                }
                None => port_position.y + label_position.y,
            };

            loop {
                let mut adjusted = false;
                for (placed_x, placed_y, placed_w, placed_h) in &placed_rectangles {
                    let horizontal_overlap = absolute_x < *placed_x + *placed_w + config.gap_x
                        && absolute_x + label_size.x > *placed_x - config.gap_x;
                    let vertical_overlap = absolute_y < *placed_y + *placed_h + config.gap_y
                        && absolute_y + label_size.y > *placed_y - config.gap_y;

                    if horizontal_overlap && vertical_overlap {
                        absolute_y = if config.positive_direction {
                            *placed_y + *placed_h + config.gap_y
                        } else {
                            *placed_y - label_size.y - config.gap_y
                        };
                        adjusted = true;
                    }
                }

                if !adjusted {
                    break;
                }
            }

            label_position.x = absolute_x - port_position.x;
            label_position.y = absolute_y - port_position.y;
            label.set_position(label_position);
            placed_rectangles.push((absolute_x, absolute_y, label_size.x, label_size.y));
        }
    }

    fn compute_stack_start_coordinate(
        entries: &[(ElkPortAdapter, ElkLabelAdapter)],
        positive_direction: bool,
        gap_y: f64,
    ) -> Option<f64> {
        if entries.is_empty() {
            return None;
        }

        if positive_direction {
            let baseline = entries
                .iter()
                .map(|(port, _)| {
                    let position = port.get_position();
                    let size = port.get_size();
                    position.y + size.y
                })
                .fold(f64::NEG_INFINITY, f64::max);
            Some(baseline + gap_y)
        } else {
            let baseline = entries
                .iter()
                .map(|(port, _)| port.get_position().y)
                .fold(f64::INFINITY, f64::min);
            Some(baseline - gap_y)
        }
    }

    fn inside_horizontal_label_clamp_bounds_generic<T, N>(node: &N, node_width: f64) -> (f64, f64)
    where
        N: NodeAdapter<T>,
    {
        let width = node_width.max(0.0);
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = width;

        if node.has_property(CoreOptions::PADDING) {
            let padding = node.get_padding();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }
        if node.has_property(CoreOptions::SPACING_LABEL_NODE) {
            let spacing = node
                .get_property(CoreOptions::SPACING_LABEL_NODE)
                .unwrap_or(0.0)
                .max(0.0);
            min_x = min_x.max(spacing);
            max_x = (width - spacing).max(min_x);
        }
        if node.has_property(CoreOptions::NODE_LABELS_PADDING) {
            let padding = node
                .get_property(CoreOptions::NODE_LABELS_PADDING)
                .unwrap_or_default();
            min_x = min_x.max(padding.left.max(0.0));
            max_x = (width - padding.right.max(0.0)).max(min_x);
        }

        (min_x, max_x)
    }

    fn stack_horizontal_side_labels_generic<T, N>(
        node: &N,
        entries: &[(usize, usize)],
        config: HorizontalStackConfig,
    ) where
        N: NodeAdapter<T>,
    {
        if entries.is_empty() {
            return;
        }

        if entries.len() <= 2 && config.clamp_x.is_none() {
            return;
        }

        let ports = node.get_ports();

        // Build (port, label, absolute_x) tuples for sorting
        let mut sortable_entries: Vec<(usize, usize, f64)> = Vec::new();
        for &(port_idx, label_idx) in entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];
            let port_position = port.get_position();
            let label_position = label.get_position();
            let absolute_x = port_position.x + label_position.x;
            sortable_entries.push((port_idx, label_idx, absolute_x));
        }

        sortable_entries.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

        let mut placed_rectangles: Vec<(f64, f64, f64, f64)> = Vec::new();
        for (port_idx, label_idx, _) in sortable_entries {
            let port = &ports[port_idx];
            let labels = port.get_labels();
            let label = &labels[label_idx];

            let port_position = port.get_position();
            let port_size = port.get_size();
            let label_size = label.get_size();
            let mut label_position = label.get_position();

            let mut absolute_x = port_position.x + label_position.x;
            if let Some((min_x, max_x)) = config.clamp_x {
                let actual_min_x = min_x.min(port_position.x);
                let actual_max_x = max_x.max(port_position.x + port_size.x);
                if absolute_x < actual_min_x {
                    absolute_x = actual_min_x;
                } else if absolute_x + label_size.x > actual_max_x {
                    absolute_x = actual_max_x - label_size.x;
                }
            }
            let mut absolute_y = match config.start_coordinate {
                Some(start) => {
                    if config.positive_direction {
                        start
                    } else {
                        start - label_size.y
                    }
                }
                None => port_position.y + label_position.y,
            };

            loop {
                let mut adjusted = false;
                for (placed_x, placed_y, placed_w, placed_h) in &placed_rectangles {
                    let horizontal_overlap = absolute_x < *placed_x + *placed_w + config.gap_x
                        && absolute_x + label_size.x > *placed_x - config.gap_x;
                    let vertical_overlap = absolute_y < *placed_y + *placed_h + config.gap_y
                        && absolute_y + label_size.y > *placed_y - config.gap_y;

                    if horizontal_overlap && vertical_overlap {
                        absolute_y = if config.positive_direction {
                            *placed_y + *placed_h + config.gap_y
                        } else {
                            *placed_y - label_size.y - config.gap_y
                        };
                        adjusted = true;
                    }
                }

                if !adjusted {
                    break;
                }
            }

            label_position.x = absolute_x - port_position.x;
            label_position.y = absolute_y - port_position.y;
            label.set_position(label_position);
            placed_rectangles.push((absolute_x, absolute_y, label_size.x, label_size.y));
        }
    }

    fn compute_stack_start_coordinate_generic<T, N>(
        node: &N,
        entries: &[(usize, usize)],
        positive_direction: bool,
        gap_y: f64,
    ) -> Option<f64>
    where
        N: NodeAdapter<T>,
    {
        if entries.is_empty() {
            return None;
        }

        let ports = node.get_ports();

        if positive_direction {
            let baseline = entries
                .iter()
                .map(|(port_idx, _)| {
                    let port = &ports[*port_idx];
                    let position = port.get_position();
                    let size = port.get_size();
                    position.y + size.y
                })
                .fold(f64::NEG_INFINITY, f64::max);
            Some(baseline + gap_y)
        } else {
            let baseline = entries
                .iter()
                .map(|(port_idx, _)| {
                    let port = &ports[*port_idx];
                    port.get_position().y
                })
                .fold(f64::INFINITY, f64::min);
            Some(baseline - gap_y)
        }
    }

}

#[derive(Clone, Copy)]
enum LabelPlacementRelation {
    Centered,
    BelowOrRight,
    AboveOrLeft,
}
