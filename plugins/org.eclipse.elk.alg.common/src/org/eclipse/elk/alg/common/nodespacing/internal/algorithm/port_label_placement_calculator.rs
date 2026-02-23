use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{
    ContainerArea, HorizontalLabelAlignment, VerticalLabelAlignment,
};
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::overlaps::{
    OverlapRemovalDirection, RectangleStripOverlapRemover,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortConstraints, PortLabelPlacement, PortSide, SizeConstraint,
};

use super::node_label_and_size_utilities::NodeLabelAndSizeUtilities;

/// Knows how to place port labels.
///
/// Faithfully ports Java's `PortLabelPlacementCalculator`.
pub struct PortLabelPlacementCalculator;

impl PortLabelPlacementCalculator {
    fn set_inside_north_south_label_cell_height(
        node_context: &mut NodeContext,
        port_side: PortSide,
        height: f64,
    ) {
        if !(port_side == PortSide::North || port_side == PortSide::South) {
            return;
        }

        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.minimum_content_area_size_mut().y = height;
        }

        let container_area = if port_side == PortSide::North {
            ContainerArea::Begin
        } else {
            ContainerArea::End
        };
        if let Some(cell) = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            cell.minimum_content_area_size_mut().y = height;
        }
    }

    /// Places port labels for northern and southern ports. If port labels are placed on
    /// the inside, the height required for the placement is set as the height of the
    /// content area of northern and southern inside port label cells.
    pub fn place_horizontal_port_labels(node_context: &mut NodeContext) {
        Self::place_port_labels(node_context, PortSide::North);
        Self::place_port_labels(node_context, PortSide::South);
    }

    /// Places port labels for eastern and western ports.
    pub fn place_vertical_port_labels(node_context: &mut NodeContext) {
        Self::place_port_labels(node_context, PortSide::East);
        Self::place_port_labels(node_context, PortSide::West);
    }

    fn place_port_labels(node_context: &mut NodeContext, port_side: PortSide) {
        let constrained_placement = !node_context
            .size_constraints
            .contains(&SizeConstraint::PortLabels)
            || node_context.port_constraints == PortConstraints::FixedPos;

        if node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::Inside)
        {
            if constrained_placement {
                Self::constrained_inside_port_label_placement(node_context, port_side);
            } else {
                Self::simple_inside_port_label_placement(node_context, port_side);
            }
        } else if node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::Outside)
        {
            if constrained_placement {
                Self::constrained_outside_port_label_placement(node_context, port_side);
            } else {
                Self::simple_outside_port_label_placement(node_context, port_side);
            }
        }
    }

    // ==================================================================================
    // Simple Inside Port Labels
    // ==================================================================================

    fn simple_inside_port_label_placement(node_context: &mut NodeContext, port_side: PortSide) {
        let mut inside_north_or_south_port_label_area_height = 0.0_f64;

        let label_border_offset = Self::port_label_border_offset_for_port_side(node_context, port_side);
        let port_label_spacing_horizontal = node_context.port_label_spacing_horizontal;
        let port_label_spacing_vertical = node_context.port_label_spacing_vertical;
        let port_labels_treat_as_group = node_context.port_labels_treat_as_group;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_size = port_context.port_size;
                let port_border_offset = port_context.port_border_offset;
                let labels_next_to_port = port_context.labels_next_to_port;

                let port_label_cell = port_context.port_label_cell.as_mut().unwrap();
                let min_width = port_label_cell.minimum_width();
                let min_height = port_label_cell.minimum_height();

                {
                    let rect = port_label_cell.cell_rectangle();
                    rect.width = min_width;
                    rect.height = min_height;
                }

                match port_side {
                    PortSide::North => {
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = if labels_next_to_port {
                            (port_size.x - min_width) / 2.0
                        } else {
                            port_size.x + port_label_spacing_horizontal
                        };
                        rect.y = port_size.y + port_border_offset + label_border_offset;
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Center);
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Top);
                    }
                    PortSide::South => {
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = if labels_next_to_port {
                            (port_size.x - min_width) / 2.0
                        } else {
                            port_size.x + port_label_spacing_horizontal
                        };
                        rect.y = -port_border_offset - label_border_offset - min_height;
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Center);
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                    }
                    PortSide::East => {
                        let first_label_height = if !port_labels_treat_as_group {
                            port_label_cell.labels().first().map(|l| l.get_size().y).unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = -port_border_offset - label_border_offset - min_width;
                        rect.y = if labels_next_to_port {
                            let label_height = if port_labels_treat_as_group {
                                min_height
                            } else {
                                first_label_height
                            };
                            (port_size.y - label_height) / 2.0
                        } else {
                            port_size.y + port_label_spacing_vertical
                        };
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Right);
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Center);
                    }
                    PortSide::West => {
                        let first_label_height = if !port_labels_treat_as_group {
                            port_label_cell.labels().first().map(|l| l.get_size().y).unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = port_size.x + port_border_offset + label_border_offset;
                        rect.y = if labels_next_to_port {
                            let label_height = if port_labels_treat_as_group {
                                min_height
                            } else {
                                first_label_height
                            };
                            (port_size.y - label_height) / 2.0
                        } else {
                            port_size.y + port_label_spacing_vertical
                        };
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Left);
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Center);
                    }
                    _ => {}
                }

                // Update port label area height for N/S
                if port_side == PortSide::North || port_side == PortSide::South {
                    inside_north_or_south_port_label_area_height =
                        inside_north_or_south_port_label_area_height.max(min_height);
                }
            }
        }

        // Apply N/S port label area height
        if inside_north_or_south_port_label_area_height > 0.0 {
            Self::set_inside_north_south_label_cell_height(
                node_context,
                port_side,
                inside_north_or_south_port_label_area_height,
            );
        }
    }

    fn port_label_border_offset_for_port_side(
        node_context: &NodeContext,
        port_side: PortSide,
    ) -> f64 {
        let container_padding = node_context.node_container.padding_ref();
        match port_side {
            PortSide::North => {
                container_padding.top + node_context.port_label_spacing_vertical
            }
            PortSide::South => {
                container_padding.bottom + node_context.port_label_spacing_vertical
            }
            PortSide::East => {
                container_padding.right + node_context.port_label_spacing_horizontal
            }
            PortSide::West => {
                container_padding.left + node_context.port_label_spacing_horizontal
            }
            _ => 0.0,
        }
    }

    // ==================================================================================
    // Constrained Inside Port Labels
    // ==================================================================================

    fn constrained_inside_port_label_placement(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        // For East/West, simply revert to simple placement
        if port_side == PortSide::East || port_side == PortSide::West {
            Self::simple_inside_port_label_placement(node_context, port_side);
            return;
        }

        let overlap_removal_direction = if port_side == PortSide::North {
            OverlapRemovalDirection::Down
        } else {
            OverlapRemovalDirection::Up
        };
        let vertical_label_alignment = if port_side == PortSide::North {
            VerticalLabelAlignment::Top
        } else {
            VerticalLabelAlignment::Bottom
        };

        // Get cell boundaries
        let (left_border, right_border) =
            if let Some(cell) = node_context.inside_port_label_cells.get(&port_side) {
                let label_container_rect = *cell.cell_rectangle_ref();
                let cell_padding = cell.padding_ref();
                let left = label_container_rect.x
                    + cell_padding
                        .left
                        .max(node_context.surrounding_port_margins.left)
                        .max(node_context.node_label_spacing);
                let right = label_container_rect.x + label_container_rect.width
                    - cell_padding
                        .right
                        .max(node_context.surrounding_port_margins.right)
                        .max(node_context.node_label_spacing);
                (left, right)
            } else {
                return;
            };

        let port_label_spacing_h = node_context.port_label_spacing_horizontal;
        let port_label_spacing_v = node_context.port_label_spacing_vertical;

        // Prepare overlap remover
        let mut overlap_remover =
            RectangleStripOverlapRemover::create_for_direction(overlap_removal_direction)
                .with_gap(port_label_spacing_h, port_label_spacing_v);

        // First pass: set up rectangle positions and add to overlap remover
        let mut start_coordinate = if port_side == PortSide::North {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_size = port_context.port_size;
                let port_position = port_context.port_position;

                let port_label_cell = port_context.port_label_cell.as_mut().unwrap();
                let min_width = port_label_cell.minimum_width();
                let min_height = port_label_cell.minimum_height();

                {
                    let rect = port_label_cell.cell_rectangle();
                    rect.width = min_width;
                    rect.height = min_height;
                }

                port_label_cell.set_vertical_alignment(vertical_label_alignment);
                port_label_cell.set_horizontal_alignment(HorizontalLabelAlignment::Right);

                // Center the label, keeping it within boundaries
                Self::center_port_label(
                    port_label_cell.cell_rectangle(),
                    &port_position,
                    &port_size,
                    left_border,
                    right_border,
                );

                // Add to overlap remover
                overlap_remover.add_rectangle(port_label_cell.cell_rectangle());

                // Update start coordinate
                if port_side == PortSide::North {
                    start_coordinate =
                        start_coordinate.max(port_position.y + port_size.y);
                } else {
                    start_coordinate = start_coordinate.min(port_position.y);
                }
            }
        }

        // Offset start coordinate by port-label spacing
        start_coordinate += if port_side == PortSide::North {
            port_label_spacing_v
        } else {
            -port_label_spacing_v
        };

        // Invoke overlap removal
        let strip_height = overlap_remover
            .with_start_coordinate(start_coordinate)
            .remove_overlaps();

        if strip_height > 0.0 {
            Self::set_inside_north_south_label_cell_height(node_context, port_side, strip_height);
        }

        // Second pass: convert label cell coordinates to be relative to port positions
        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_position = port_context.port_position;
                let rect = port_context
                    .port_label_cell
                    .as_mut()
                    .unwrap()
                    .cell_rectangle();
                rect.x -= port_position.x;
                rect.y -= port_position.y;
            }
        }
    }

    fn center_port_label(
        port_label_cell_rect: &mut ElkRectangle,
        port_position: &org_eclipse_elk_core::org::eclipse::elk::core::math::KVector,
        port_size: &org_eclipse_elk_core::org::eclipse::elk::core::math::KVector,
        min_x: f64,
        max_x: f64,
    ) {
        // Center the label
        port_label_cell_rect.x =
            port_position.x - (port_label_cell_rect.width - port_size.x) / 2.0;

        // Make sure it doesn't slide past the port
        let actual_min_x = min_x.min(port_position.x);
        let actual_max_x = max_x.max(port_position.x + port_size.x);

        // Keep inside boundaries
        if port_label_cell_rect.x < actual_min_x {
            port_label_cell_rect.x = actual_min_x;
        } else if port_label_cell_rect.x + port_label_cell_rect.width > actual_max_x {
            port_label_cell_rect.x = actual_max_x - port_label_cell_rect.width;
        }
    }

    // ==================================================================================
    // Simple Outside Port Labels
    // ==================================================================================

    fn simple_outside_port_label_placement(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let place_first_port_differently =
            NodeLabelAndSizeUtilities::is_first_outside_port_label_placed_differently(
                node_context,
                port_side,
            );

        let always_above = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::AlwaysOtherSameSide);

        let port_label_spacing_h = node_context.port_label_spacing_horizontal;
        let port_label_spacing_v = node_context.port_label_spacing_vertical;
        let port_labels_treat_as_group = node_context.port_labels_treat_as_group;

        let mut first_special = place_first_port_differently;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_size = port_context.port_size;
                let labels_next_to_port = port_context.labels_next_to_port;

                let port_label_cell = port_context.port_label_cell.as_mut().unwrap();
                let min_width = port_label_cell.minimum_width();
                let min_height = port_label_cell.minimum_height();

                {
                    let rect = port_label_cell.cell_rectangle();
                    rect.width = min_width;
                    rect.height = min_height;
                }

                match port_side {
                    PortSide::North => {
                        let rect = port_label_cell.cell_rectangle();
                        if labels_next_to_port {
                            rect.x = (port_size.x - min_width) / 2.0;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Center);
                        } else if first_special || always_above {
                            rect.x = -min_width - port_label_spacing_h;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Right);
                        } else {
                            rect.x = port_size.x + port_label_spacing_h;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Left);
                        }
                        let rect = port_label_cell.cell_rectangle();
                        rect.y = -min_height - port_label_spacing_v;
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                    }
                    PortSide::South => {
                        let rect = port_label_cell.cell_rectangle();
                        if labels_next_to_port {
                            rect.x = (port_size.x - min_width) / 2.0;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Center);
                        } else if first_special || always_above {
                            rect.x = -min_width - port_label_spacing_h;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Right);
                        } else {
                            rect.x = port_size.x + port_label_spacing_h;
                            port_label_cell
                                .set_horizontal_alignment(HorizontalLabelAlignment::Left);
                        }
                        let rect = port_label_cell.cell_rectangle();
                        rect.y = port_size.y + port_label_spacing_v;
                        port_label_cell.set_vertical_alignment(VerticalLabelAlignment::Top);
                    }
                    PortSide::East => {
                        let first_label_height = if !port_labels_treat_as_group {
                            port_label_cell.labels().first().map(|l| l.get_size().y).unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        let rect = port_label_cell.cell_rectangle();
                        if labels_next_to_port {
                            let label_height = if port_labels_treat_as_group {
                                min_height
                            } else {
                                first_label_height
                            };
                            rect.y = (port_size.y - label_height) / 2.0;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Center);
                        } else if first_special || always_above {
                            rect.y = -min_height - port_label_spacing_v;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Bottom);
                        } else {
                            rect.y = port_size.y + port_label_spacing_v;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Top);
                        }
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = port_size.x + port_label_spacing_h;
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Left);
                    }
                    PortSide::West => {
                        let first_label_height = if !port_labels_treat_as_group {
                            port_label_cell.labels().first().map(|l| l.get_size().y).unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        let rect = port_label_cell.cell_rectangle();
                        if labels_next_to_port {
                            let label_height = if port_labels_treat_as_group {
                                min_height
                            } else {
                                first_label_height
                            };
                            rect.y = (port_size.y - label_height) / 2.0;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Center);
                        } else if first_special || always_above {
                            rect.y = -min_height - port_label_spacing_v;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Bottom);
                        } else {
                            rect.y = port_size.y + port_label_spacing_v;
                            port_label_cell
                                .set_vertical_alignment(VerticalLabelAlignment::Top);
                        }
                        let rect = port_label_cell.cell_rectangle();
                        rect.x = -min_width - port_label_spacing_h;
                        port_label_cell
                            .set_horizontal_alignment(HorizontalLabelAlignment::Right);
                    }
                    _ => {}
                }

                // The next port doesn't have special needs
                first_special = false;
            }
        }
    }

    // ==================================================================================
    // Constrained Outside Port Labels
    // ==================================================================================

    fn constrained_outside_port_label_placement(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let port_count = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.len())
            .unwrap_or(0);

        // If at most 2 ports or E/W, revert to simple placement
        if port_count <= 2 || port_side == PortSide::East || port_side == PortSide::West {
            Self::simple_outside_port_label_placement(node_context, port_side);
            return;
        }

        let mut port_with_special_needs = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::SpaceEfficient);

        let overlap_removal_direction = if port_side == PortSide::North {
            OverlapRemovalDirection::Up
        } else {
            OverlapRemovalDirection::Down
        };
        let vertical_label_alignment = if port_side == PortSide::North {
            VerticalLabelAlignment::Bottom
        } else {
            VerticalLabelAlignment::Top
        };

        let port_label_spacing_h = node_context.port_label_spacing_horizontal;
        let port_label_spacing_v = node_context.port_label_spacing_vertical;

        // Prepare overlap remover
        let mut overlap_remover =
            RectangleStripOverlapRemover::create_for_direction(overlap_removal_direction)
                .with_gap(port_label_spacing_v, port_label_spacing_h);

        let mut start_coordinate = if port_side == PortSide::North {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_size = port_context.port_size;
                let port_position = port_context.port_position;

                let port_label_cell = port_context.port_label_cell.as_mut().unwrap();
                let min_width = port_label_cell.minimum_width();
                let min_height = port_label_cell.minimum_height();

                {
                    let rect = port_label_cell.cell_rectangle();
                    rect.width = min_width;
                    rect.height = min_height;

                    if port_with_special_needs {
                        rect.x = port_position.x - min_width - port_label_spacing_h;
                        port_with_special_needs = false;
                    } else {
                        rect.x = port_position.x + port_size.x + port_label_spacing_h;
                    }
                }

                port_label_cell.set_vertical_alignment(vertical_label_alignment);
                port_label_cell.set_horizontal_alignment(HorizontalLabelAlignment::Right);

                // Add to overlap remover
                overlap_remover.add_rectangle(port_label_cell.cell_rectangle());

                // Update start coordinate
                if port_side == PortSide::North {
                    start_coordinate = start_coordinate.min(port_position.y);
                } else {
                    start_coordinate =
                        start_coordinate.max(port_position.y + port_size.y);
                }
            }
        }

        // Offset start coordinate
        start_coordinate += if port_side == PortSide::North {
            -port_label_spacing_v
        } else {
            port_label_spacing_v
        };

        // Invoke overlap removal
        overlap_remover
            .with_start_coordinate(start_coordinate)
            .remove_overlaps();

        // Convert coordinates to be relative to port positions
        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let has_labels = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.has_labels())
                    .unwrap_or(false);
                if !has_labels {
                    continue;
                }

                let port_position = port_context.port_position;
                let rect = port_context
                    .port_label_cell
                    .as_mut()
                    .unwrap()
                    .cell_rectangle();
                rect.x -= port_position.x;
                rect.y -= port_position.y;
            }
        }
    }
}
