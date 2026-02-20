use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::port_context::PortContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortAlignment, PortConstraints, PortSide, SizeOptions,
};

/// Actually places ports.
///
/// Faithfully ports Java's `PortPlacementCalculator`.
pub struct PortPlacementCalculator;

impl PortPlacementCalculator {
    // ==================================================================================
    // Horizontal Port Placement
    // ==================================================================================

    /// Places horizontal ports (northern and southern).
    pub fn place_horizontal_ports(node_context: &mut NodeContext) {
        match node_context.port_constraints {
            PortConstraints::FixedPos => {
                Self::place_horizontal_fixed_pos_ports(node_context, PortSide::North);
                Self::place_horizontal_fixed_pos_ports(node_context, PortSide::South);
            }
            PortConstraints::FixedRatio => {
                Self::place_horizontal_fixed_ratio_ports(node_context, PortSide::North);
                Self::place_horizontal_fixed_ratio_ports(node_context, PortSide::South);
            }
            _ => {
                Self::place_horizontal_free_ports(node_context, PortSide::North);
                Self::place_horizontal_free_ports(node_context, PortSide::South);
            }
        }
    }

    fn place_horizontal_fixed_pos_ports(node_context: &mut NodeContext, port_side: PortSide) {
        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.y =
                    Self::calculate_horizontal_port_y_coordinate(port_context);
            }
        }
    }

    fn place_horizontal_fixed_ratio_ports(node_context: &mut NodeContext, port_side: PortSide) {
        let node_width = node_context.node_size.x;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.x =
                    node_width * port_context.port_ratio_or_position;
                port_context.port_position.y =
                    Self::calculate_horizontal_port_y_coordinate(port_context);
            }
        }
    }

    fn place_horizontal_free_ports(node_context: &mut NodeContext, port_side: PortSide) {
        let port_count = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.len())
            .unwrap_or(0);
        if port_count == 0 {
            return;
        }

        // Read cell information
        let (cell_rect, cell_padding, min_content_x) =
            if let Some(cell) = node_context.inside_port_label_cells.get(&port_side) {
                (
                    *cell.cell_rectangle_ref(),
                    cell.padding_ref().clone(),
                    cell.minimum_content_area_size().x,
                )
            } else {
                return;
            };

        let mut port_alignment = node_context.get_port_alignment(port_side);
        let available_space = cell_rect.width - cell_padding.left - cell_padding.right;
        let mut calculated_port_placement_width = min_content_x;
        let mut current_x_pos = cell_rect.x + cell_padding.left;
        let mut space_between_ports = node_context.port_port_spacing;

        // If DISTRIBUTED or JUSTIFIED with single port, switch to CENTER
        if (port_alignment == PortAlignment::Distributed
            || port_alignment == PortAlignment::Justified)
            && port_count == 1
        {
            calculated_port_placement_width = Self::modified_port_placement_size(
                node_context,
                port_alignment,
                calculated_port_placement_width,
            );
            port_alignment = PortAlignment::Center;
        }

        if available_space < calculated_port_placement_width
            && !node_context
                .size_options
                .contains(&SizeOptions::PortsOverhang)
        {
            // Not enough space and ports can't overhang - cram them in
            if port_alignment == PortAlignment::Distributed {
                space_between_ports += (available_space - calculated_port_placement_width)
                    / (port_count + 1) as f64;
                current_x_pos += space_between_ports;
            } else {
                space_between_ports += (available_space - calculated_port_placement_width)
                    / (port_count - 1) as f64;
            }
        } else {
            // We are allowed to overhang
            if available_space < calculated_port_placement_width {
                calculated_port_placement_width = Self::modified_port_placement_size(
                    node_context,
                    port_alignment,
                    calculated_port_placement_width,
                );
                port_alignment = PortAlignment::Center;
            }

            match port_alignment {
                PortAlignment::Begin => { /* nothing */ }
                PortAlignment::Center => {
                    current_x_pos +=
                        (available_space - calculated_port_placement_width) / 2.0;
                }
                PortAlignment::End => {
                    current_x_pos += available_space - calculated_port_placement_width;
                }
                PortAlignment::Distributed => {
                    let additional = (available_space - calculated_port_placement_width)
                        / (port_count + 1) as f64;
                    space_between_ports += 0.0_f64.max(additional);
                    current_x_pos += space_between_ports;
                }
                PortAlignment::Justified => {
                    let additional = (available_space - calculated_port_placement_width)
                        / (port_count - 1) as f64;
                    space_between_ports += 0.0_f64.max(additional);
                }
            }
        }

        // Iterate over all ports and place them
        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.x = current_x_pos + port_context.port_margin.left;
                port_context.port_position.y =
                    Self::calculate_horizontal_port_y_coordinate(port_context);

                current_x_pos += port_context.port_margin.left
                    + port_context.port_size.x
                    + port_context.port_margin.right
                    + space_between_ports;
            }
        }
    }

    fn calculate_horizontal_port_y_coordinate(port_context: &PortContext) -> f64 {
        if port_context.has_port_border_offset {
            if port_context.port_side == PortSide::North {
                -port_context.port_size.y - port_context.port_border_offset
            } else {
                port_context.port_border_offset
            }
        } else if port_context.port_side == PortSide::North {
            -port_context.port_size.y
        } else {
            0.0
        }
    }

    // ==================================================================================
    // Vertical Port Placement
    // ==================================================================================

    /// Places vertical ports (eastern and western).
    pub fn place_vertical_ports(node_context: &mut NodeContext) {
        match node_context.port_constraints {
            PortConstraints::FixedPos => {
                Self::place_vertical_fixed_pos_ports(node_context, PortSide::East);
                Self::place_vertical_fixed_pos_ports(node_context, PortSide::West);
            }
            PortConstraints::FixedRatio => {
                Self::place_vertical_fixed_ratio_ports(node_context, PortSide::East);
                Self::place_vertical_fixed_ratio_ports(node_context, PortSide::West);
            }
            _ => {
                Self::place_vertical_free_ports(node_context, PortSide::East);
                Self::place_vertical_free_ports(node_context, PortSide::West);
            }
        }
    }

    fn place_vertical_fixed_pos_ports(node_context: &mut NodeContext, port_side: PortSide) {
        let node_width = node_context.node_size.x;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.x =
                    Self::calculate_vertical_port_x_coordinate(port_context, node_width);
            }
        }
    }

    fn place_vertical_fixed_ratio_ports(node_context: &mut NodeContext, port_side: PortSide) {
        let node_width = node_context.node_size.x;
        let node_height = node_context.node_size.y;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.x =
                    Self::calculate_vertical_port_x_coordinate(port_context, node_width);
                port_context.port_position.y =
                    node_height * port_context.port_ratio_or_position;
            }
        }
    }

    fn place_vertical_free_ports(node_context: &mut NodeContext, port_side: PortSide) {
        let port_count = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.len())
            .unwrap_or(0);
        if port_count == 0 {
            return;
        }

        // Read cell information
        let (cell_rect, cell_padding, min_content_y) =
            if let Some(cell) = node_context.inside_port_label_cells.get(&port_side) {
                (
                    *cell.cell_rectangle_ref(),
                    cell.padding_ref().clone(),
                    cell.minimum_content_area_size().y,
                )
            } else {
                return;
            };

        let mut port_alignment = node_context.get_port_alignment(port_side);
        let available_space = cell_rect.height - cell_padding.top - cell_padding.bottom;
        let mut calculated_port_placement_height = min_content_y;
        let mut current_y_pos = cell_rect.y + cell_padding.top;
        let mut space_between_ports = node_context.port_port_spacing;
        let node_width = node_context.node_size.x;

        // If DISTRIBUTED or JUSTIFIED with single port, switch to CENTER
        if (port_alignment == PortAlignment::Distributed
            || port_alignment == PortAlignment::Justified)
            && port_count == 1
        {
            calculated_port_placement_height = Self::modified_port_placement_size(
                node_context,
                port_alignment,
                calculated_port_placement_height,
            );
            port_alignment = PortAlignment::Center;
        }

        if available_space < calculated_port_placement_height
            && !node_context
                .size_options
                .contains(&SizeOptions::PortsOverhang)
        {
            // Not enough space and ports can't overhang - cram them in
            if port_alignment == PortAlignment::Distributed {
                space_between_ports += (available_space - calculated_port_placement_height)
                    / (port_count + 1) as f64;
                current_y_pos += space_between_ports;
            } else {
                space_between_ports += (available_space - calculated_port_placement_height)
                    / (port_count - 1) as f64;
            }
        } else {
            // We are allowed to overhang
            if available_space < calculated_port_placement_height {
                calculated_port_placement_height = Self::modified_port_placement_size(
                    node_context,
                    port_alignment,
                    calculated_port_placement_height,
                );
                port_alignment = PortAlignment::Center;
            }

            match port_alignment {
                PortAlignment::Begin => { /* nothing */ }
                PortAlignment::Center => {
                    current_y_pos +=
                        (available_space - calculated_port_placement_height) / 2.0;
                }
                PortAlignment::End => {
                    current_y_pos += available_space - calculated_port_placement_height;
                }
                PortAlignment::Distributed => {
                    let additional = (available_space - calculated_port_placement_height)
                        / (port_count + 1) as f64;
                    space_between_ports += 0.0_f64.max(additional);
                    current_y_pos += space_between_ports;
                }
                PortAlignment::Justified => {
                    let additional = (available_space - calculated_port_placement_height)
                        / (port_count - 1) as f64;
                    space_between_ports += 0.0_f64.max(additional);
                }
            }
        }

        // Iterate over all ports and place them
        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                port_context.port_position.x =
                    Self::calculate_vertical_port_x_coordinate(port_context, node_width);
                port_context.port_position.y = current_y_pos + port_context.port_margin.top;

                current_y_pos += port_context.port_margin.top
                    + port_context.port_size.y
                    + port_context.port_margin.bottom
                    + space_between_ports;
            }
        }
    }

    fn calculate_vertical_port_x_coordinate(
        port_context: &PortContext,
        node_width: f64,
    ) -> f64 {
        if port_context.has_port_border_offset {
            if port_context.port_side == PortSide::West {
                -port_context.port_size.x - port_context.port_border_offset
            } else {
                node_width + port_context.port_border_offset
            }
        } else if port_context.port_side == PortSide::West {
            -port_context.port_size.x
        } else {
            node_width
        }
    }

    // ==================================================================================
    // Utilities
    // ==================================================================================

    fn modified_port_placement_size(
        node_context: &NodeContext,
        old_port_alignment: PortAlignment,
        current_port_placement_size: f64,
    ) -> f64 {
        if old_port_alignment == PortAlignment::Distributed {
            current_port_placement_size - 2.0 * node_context.port_port_spacing
        } else {
            current_port_placement_size
        }
    }
}
