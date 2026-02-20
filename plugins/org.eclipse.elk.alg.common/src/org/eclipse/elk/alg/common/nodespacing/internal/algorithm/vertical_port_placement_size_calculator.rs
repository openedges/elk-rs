use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::ContainerArea;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::port_context::PortContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortAlignment, PortLabelPlacement, PortSide, SizeConstraint, SizeOptions,
};

use super::horizontal_port_placement_size_calculator::HorizontalPortPlacementSizeCalculator;

/// Calculates the space required to set up vertical port placements (for E/W ports).
///
/// Faithfully ports Java's `VerticalPortPlacementSizeCalculator`.
pub struct VerticalPortPlacementSizeCalculator;

impl VerticalPortPlacementSizeCalculator {
    pub fn calculate_vertical_port_placement_size(node_context: &mut NodeContext) {
        match node_context.port_constraints {
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortConstraints::FixedPos => {
                Self::calculate_vertical_node_size_required_by_fixed_pos_ports(
                    node_context,
                    PortSide::East,
                );
                Self::calculate_vertical_node_size_required_by_fixed_pos_ports(
                    node_context,
                    PortSide::West,
                );
            }
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortConstraints::FixedRatio => {
                Self::calculate_vertical_node_size_required_by_fixed_ratio_ports(
                    node_context,
                    PortSide::East,
                );
                Self::calculate_vertical_node_size_required_by_fixed_ratio_ports(
                    node_context,
                    PortSide::West,
                );
            }
            _ => {
                Self::calculate_vertical_node_size_required_by_free_ports(
                    node_context,
                    PortSide::East,
                );
                Self::calculate_vertical_node_size_required_by_free_ports(
                    node_context,
                    PortSide::West,
                );
            }
        }
    }

    fn calculate_vertical_node_size_required_by_fixed_pos_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let mut bottommost_port_border = 0.0_f64;

        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            for port_context in port_contexts {
                bottommost_port_border = bottommost_port_border
                    .max(port_context.port_position.y + port_context.port_size.y);
            }
        }

        // Set the cell size and remove top padding
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().top = 0.0;
            cell.minimum_content_area_size_mut().y = bottommost_port_border;
        }

        // Also update the corresponding cell in the node container tree
        let container_area = match port_side {
            PortSide::East => ContainerArea::End,
            PortSide::West => ContainerArea::Begin,
            _ => return,
        };
        if let Some(atomic) = node_context
            .node_container_middle_row_mut()
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().top = 0.0;
            atomic.minimum_content_area_size_mut().y = bottommost_port_border;
        }
    }

    fn calculate_vertical_node_size_required_by_fixed_ratio_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let port_contexts_empty = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.is_empty())
            .unwrap_or(true);

        if port_contexts_empty {
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
                cell.padding().top = 0.0;
                cell.padding().bottom = 0.0;
            }
            let container_area = match port_side {
                PortSide::East => ContainerArea::End,
                PortSide::West => ContainerArea::Begin,
                _ => return,
            };
            if let Some(atomic) = node_context
                .node_container_middle_row_mut()
                .get_cell_mut(container_area)
                .as_atomic_mut()
            {
                atomic.padding().top = 0.0;
                atomic.padding().bottom = 0.0;
            }
            return;
        }

        let port_labels_inside = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::Inside);

        if node_context
            .size_constraints
            .contains(&SizeConstraint::PortLabels)
        {
            Self::setup_port_margins(node_context, port_side);
        }

        let mut min_height = 0.0_f64;
        let surrounding_top = node_context.surrounding_port_margins.top;
        let surrounding_bottom = node_context.surrounding_port_margins.bottom;
        let port_port_spacing = node_context.port_port_spacing;

        // Collect port data for iteration
        let port_data: Vec<(f64, f64, f64, f64)> = node_context
            .port_contexts
            .get(&port_side)
            .unwrap()
            .iter()
            .map(|pc| {
                (
                    pc.port_ratio_or_position,
                    pc.port_size.y,
                    pc.port_margin.top,
                    pc.port_margin.bottom,
                )
            })
            .collect();

        let mut previous: Option<(f64, f64, f64)> = None; // (ratio, height, margin_bottom)

        for &(ratio, height, margin_top, margin_bottom) in port_data.iter() {
            if let Some((prev_ratio, prev_height, prev_margin_bottom)) = previous {
                let required_space =
                    prev_height + prev_margin_bottom + port_port_spacing + margin_top;
                min_height = min_height.max(
                    HorizontalPortPlacementSizeCalculator::min_size_required_to_respect_spacing(
                        required_space,
                        prev_ratio,
                        ratio,
                    ),
                );
            } else {
                // First port
                if surrounding_top > 0.0 {
                    min_height = min_height.max(
                        HorizontalPortPlacementSizeCalculator::min_size_required_to_respect_spacing(
                            surrounding_top + margin_top,
                            0.0,
                            ratio,
                        ),
                    );
                }
            }

            previous = Some((ratio, height, margin_bottom));
        }

        // Bottom surrounding port margins
        if let Some((prev_ratio, prev_height, prev_margin_bottom)) = previous {
            if surrounding_bottom > 0.0 {
                let mut required_space = prev_height + surrounding_bottom;
                if port_labels_inside {
                    required_space += prev_margin_bottom;
                }
                min_height = min_height.max(
                    HorizontalPortPlacementSizeCalculator::min_size_required_to_respect_spacing(
                        required_space,
                        prev_ratio,
                        1.0,
                    ),
                );
            }
        }

        // Set the cell size
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().top = 0.0;
            cell.minimum_content_area_size_mut().y = min_height;
        }
        let container_area = match port_side {
            PortSide::East => ContainerArea::End,
            PortSide::West => ContainerArea::Begin,
            _ => return,
        };
        if let Some(atomic) = node_context
            .node_container_middle_row_mut()
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().top = 0.0;
            atomic.minimum_content_area_size_mut().y = min_height;
        }
    }

    fn calculate_vertical_node_size_required_by_free_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let port_contexts_empty = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.is_empty())
            .unwrap_or(true);

        let container_area = match port_side {
            PortSide::East => ContainerArea::End,
            PortSide::West => ContainerArea::Begin,
            _ => return,
        };

        if port_contexts_empty {
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
                cell.padding().top = 0.0;
                cell.padding().bottom = 0.0;
            }
            if let Some(atomic) = node_context
                .node_container_middle_row_mut()
                .get_cell_mut(container_area)
                .as_atomic_mut()
            {
                atomic.padding().top = 0.0;
                atomic.padding().bottom = 0.0;
            }
            return;
        }

        // Set the padding to match the surrounding port space
        let surr_top = node_context.surrounding_port_margins.top;
        let surr_bottom = node_context.surrounding_port_margins.bottom;

        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().top = surr_top;
            cell.padding().bottom = surr_bottom;
        }
        if let Some(atomic) = node_context
            .node_container_middle_row_mut()
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().top = surr_top;
            atomic.padding().bottom = surr_bottom;
        }

        if node_context
            .size_constraints
            .contains(&SizeConstraint::PortLabels)
        {
            Self::setup_port_margins(node_context, port_side);
        }

        let mut height = Self::port_height_plus_port_port_spacing(node_context, port_side);

        if node_context.get_port_alignment(port_side) == PortAlignment::Distributed {
            height += 2.0 * node_context.port_port_spacing;
        }

        // Set the cell size
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.minimum_content_area_size_mut().y = height;
        }
        if let Some(atomic) = node_context
            .node_container_middle_row_mut()
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.minimum_content_area_size_mut().y = height;
        }
    }

    fn setup_port_margins(node_context: &mut NodeContext, port_side: PortSide) {
        let port_labels_outside = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::Outside);
        let always_same_side = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::AlwaysSameSide);
        let always_same_side_above = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::AlwaysOtherSameSide);
        let space_efficient = node_context
            .port_labels_placement
            .contains(&PortLabelPlacement::SpaceEfficient);
        let uniform_port_spacing = node_context
            .size_options
            .contains(&SizeOptions::UniformPortSpacing);

        let port_count = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.len())
            .unwrap_or(0);

        let space_efficient_port_labels =
            !always_same_side && !always_same_side_above && (space_efficient || port_count == 2);

        // Set the vertical port margins
        Self::compute_vertical_port_margins(node_context, port_side, port_labels_outside);

        if port_labels_outside {
            if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
                let len = port_contexts.len();
                if len > 0 {
                    // Topmost port doesn't need top margin, bottommost doesn't need bottom margin
                    port_contexts[0].port_margin.top = 0.0;
                    port_contexts[len - 1].port_margin.bottom = 0.0;

                    // Space-efficient: topmost port doesn't need bottom margin if label not next to it
                    if space_efficient_port_labels && !port_contexts[0].labels_next_to_port {
                        port_contexts[0].port_margin.bottom = 0.0;
                    }
                }
            }
        }

        if uniform_port_spacing {
            if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
                Self::unify_port_margins(port_contexts);

                if port_labels_outside && !port_contexts.is_empty() {
                    let len = port_contexts.len();
                    port_contexts[0].port_margin.top = 0.0;
                    port_contexts[len - 1].port_margin.bottom = 0.0;
                }
            }
        }
    }

    fn compute_vertical_port_margins(
        node_context: &mut NodeContext,
        port_side: PortSide,
        _port_labels_outside: bool,
    ) {
        let is_fixed = PortLabelPlacement::is_fixed(&node_context.port_labels_placement);
        let port_label_spacing_v = node_context.port_label_spacing_vertical;
        let port_labels_treat_as_group = node_context.port_labels_treat_as_group;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let label_height = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.minimum_height())
                    .unwrap_or(0.0);

                if label_height > 0.0 {
                    if port_context.labels_next_to_port {
                        let port_height = port_context.port_size.y;
                        if label_height > port_height {
                            let label_count = port_context
                                .port_label_cell
                                .as_ref()
                                .map(|c| c.labels().len())
                                .unwrap_or(0);

                            if port_labels_treat_as_group || label_count == 1 {
                                // Center all labels
                                let overhang = (label_height - port_height) / 2.0;
                                port_context.port_margin.top = overhang;
                                port_context.port_margin.bottom = overhang;
                            } else {
                                // Simulate centering the first port label
                                let first_label_height = port_context
                                    .port_label_cell
                                    .as_ref()
                                    .map(|c| c.labels()[0].get_size().y)
                                    .unwrap_or(0.0);
                                let first_label_overhang =
                                    (first_label_height - port_height) / 2.0;

                                port_context.port_margin.top =
                                    0.0_f64.max(first_label_overhang);
                                port_context.port_margin.bottom =
                                    label_height - first_label_overhang - port_height;
                            }
                        }
                    } else {
                        // Label placed below the port
                        port_context.port_margin.bottom = port_label_spacing_v + label_height;
                    }
                } else if is_fixed {
                    let labels_bounds = port_context.get_labels_bounds();
                    if labels_bounds.y < 0.0 {
                        port_context.port_margin.top = -labels_bounds.y;
                    }
                    if labels_bounds.y + labels_bounds.height > port_context.port_size.y {
                        port_context.port_margin.bottom =
                            labels_bounds.y + labels_bounds.height - port_context.port_size.y;
                    }
                }
            }
        }
    }

    fn unify_port_margins(port_contexts: &mut [PortContext]) {
        let mut max_top = 0.0_f64;
        let mut max_bottom = 0.0_f64;

        for pc in port_contexts.iter() {
            max_top = max_top.max(pc.port_margin.top);
            max_bottom = max_bottom.max(pc.port_margin.bottom);
        }

        for pc in port_contexts.iter_mut() {
            pc.port_margin.top = max_top;
            pc.port_margin.bottom = max_bottom;
        }
    }

    fn port_height_plus_port_port_spacing(
        node_context: &NodeContext,
        port_side: PortSide,
    ) -> f64 {
        let mut result = 0.0;

        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            for (i, port_context) in port_contexts.iter().enumerate() {
                result +=
                    port_context.port_margin.top + port_context.port_size.y + port_context.port_margin.bottom;

                if i + 1 < port_contexts.len() {
                    result += node_context.port_port_spacing;
                }
            }
        }

        result
    }
}
