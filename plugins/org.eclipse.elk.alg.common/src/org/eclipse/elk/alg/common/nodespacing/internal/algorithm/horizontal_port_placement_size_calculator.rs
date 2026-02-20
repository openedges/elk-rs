use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::ContainerArea;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use crate::org::eclipse::elk::alg::common::nodespacing::internal::port_context::PortContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortAlignment, PortLabelPlacement, PortSide, SizeConstraint, SizeOptions,
};

const EQUALITY_TOLERANCE: f64 = 0.01;

/// Calculates the space required to set up horizontal port placements (for N/S ports).
///
/// Faithfully ports Java's `HorizontalPortPlacementSizeCalculator`.
pub struct HorizontalPortPlacementSizeCalculator;

impl HorizontalPortPlacementSizeCalculator {
    pub fn calculate_horizontal_port_placement_size(node_context: &mut NodeContext) {
        match node_context.port_constraints {
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortConstraints::FixedPos => {
                Self::calculate_horizontal_node_size_required_by_fixed_pos_ports(
                    node_context,
                    PortSide::North,
                );
                Self::calculate_horizontal_node_size_required_by_fixed_pos_ports(
                    node_context,
                    PortSide::South,
                );
            }
            org_eclipse_elk_core::org::eclipse::elk::core::options::PortConstraints::FixedRatio => {
                Self::calculate_horizontal_node_size_required_by_fixed_ratio_ports(
                    node_context,
                    PortSide::North,
                );
                Self::calculate_horizontal_node_size_required_by_fixed_ratio_ports(
                    node_context,
                    PortSide::South,
                );
            }
            _ => {
                Self::calculate_horizontal_node_size_required_by_free_ports(
                    node_context,
                    PortSide::North,
                );
                Self::calculate_horizontal_node_size_required_by_free_ports(
                    node_context,
                    PortSide::South,
                );
            }
        }
    }

    fn calculate_horizontal_node_size_required_by_fixed_pos_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let mut rightmost_port_border = 0.0_f64;

        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            for port_context in port_contexts {
                rightmost_port_border = rightmost_port_border
                    .max(port_context.port_position.x + port_context.port_size.x);
            }
        }

        // Set the cell size and remove left padding
        let container_area = match port_side {
            PortSide::North => ContainerArea::Begin,
            PortSide::South => ContainerArea::End,
            _ => return,
        };

        if let Some(atomic) = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().left = 0.0;
            atomic.minimum_content_area_size_mut().x = rightmost_port_border;
        }

        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().left = 0.0;
            cell.minimum_content_area_size_mut().x = rightmost_port_border;
        }
    }

    fn calculate_horizontal_node_size_required_by_fixed_ratio_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let port_contexts_empty = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.is_empty())
            .unwrap_or(true);

        if port_contexts_empty {
            let container_area = match port_side {
                PortSide::North => ContainerArea::Begin,
                PortSide::South => ContainerArea::End,
                _ => return,
            };
            if let Some(atomic) = node_context
                .node_container
                .get_cell_mut(container_area)
                .as_atomic_mut()
            {
                atomic.padding().left = 0.0;
                atomic.padding().right = 0.0;
            }
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
                cell.padding().left = 0.0;
                cell.padding().right = 0.0;
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

        let mut min_width = 0.0_f64;
        let surrounding_left = node_context.surrounding_port_margins.left;
        let surrounding_right = node_context.surrounding_port_margins.right;
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
                    pc.port_size.x,
                    pc.port_margin.left,
                    pc.port_margin.right,
                )
            })
            .collect();

        let mut previous: Option<(f64, f64, f64)> = None; // (ratio, width, margin_right)

        for (_i, &(ratio, width, margin_left, margin_right)) in port_data.iter().enumerate() {
            if let Some((prev_ratio, prev_width, prev_margin_right)) = previous {
                let required_space =
                    prev_width + prev_margin_right + port_port_spacing + margin_left;
                min_width = min_width.max(Self::min_size_required_to_respect_spacing(
                    required_space,
                    prev_ratio,
                    ratio,
                ));
            } else {
                // First port
                if surrounding_left > 0.0 {
                    min_width = min_width.max(Self::min_size_required_to_respect_spacing(
                        surrounding_left + margin_left,
                        0.0,
                        ratio,
                    ));
                }
            }

            previous = Some((ratio, width, margin_right));
        }

        // Right surrounding port margins
        if let Some((prev_ratio, prev_width, prev_margin_right)) = previous {
            if surrounding_right > 0.0 {
                let mut required_space = prev_width + surrounding_right;
                if port_labels_inside {
                    required_space += prev_margin_right;
                }
                min_width = min_width.max(Self::min_size_required_to_respect_spacing(
                    required_space,
                    prev_ratio,
                    1.0,
                ));
            }
        }

        // Set the cell size
        let container_area = match port_side {
            PortSide::North => ContainerArea::Begin,
            PortSide::South => ContainerArea::End,
            _ => return,
        };
        if let Some(atomic) = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().left = 0.0;
            atomic.minimum_content_area_size_mut().x = min_width;
        }
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().left = 0.0;
            cell.minimum_content_area_size_mut().x = min_width;
        }
    }

    /// Returns the minimum width that will satisfy the given spacing between the two
    /// ratios multiplied by the result width.
    pub fn min_size_required_to_respect_spacing(
        spacing: f64,
        first_ratio: f64,
        second_ratio: f64,
    ) -> f64 {
        debug_assert!(second_ratio >= first_ratio);

        if (first_ratio - second_ratio).abs() < EQUALITY_TOLERANCE {
            0.0
        } else {
            spacing / (second_ratio - first_ratio)
        }
    }

    fn calculate_horizontal_node_size_required_by_free_ports(
        node_context: &mut NodeContext,
        port_side: PortSide,
    ) {
        let port_contexts_empty = node_context
            .port_contexts
            .get(&port_side)
            .map(|v| v.is_empty())
            .unwrap_or(true);

        let container_area = match port_side {
            PortSide::North => ContainerArea::Begin,
            PortSide::South => ContainerArea::End,
            _ => return,
        };

        if port_contexts_empty {
            if let Some(atomic) = node_context
                .node_container
                .get_cell_mut(container_area)
                .as_atomic_mut()
            {
                atomic.padding().left = 0.0;
                atomic.padding().right = 0.0;
            }
            if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
                cell.padding().left = 0.0;
                cell.padding().right = 0.0;
            }
            return;
        }

        // Set the padding to match the surrounding port space
        let surr_left = node_context.surrounding_port_margins.left;
        let surr_right = node_context.surrounding_port_margins.right;

        if let Some(atomic) = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.padding().left = surr_left;
            atomic.padding().right = surr_right;
        }
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.padding().left = surr_left;
            cell.padding().right = surr_right;
        }

        if node_context
            .size_constraints
            .contains(&SizeConstraint::PortLabels)
        {
            Self::setup_port_margins(node_context, port_side);
        }

        let mut width = Self::port_width_plus_port_port_spacing(node_context, port_side);

        if node_context.get_port_alignment(port_side) == PortAlignment::Distributed {
            width += 2.0 * node_context.port_port_spacing;
        }

        // Set the cell size
        if let Some(atomic) = node_context
            .node_container
            .get_cell_mut(container_area)
            .as_atomic_mut()
        {
            atomic.minimum_content_area_size_mut().x = width;
        }
        if let Some(cell) = node_context.inside_port_label_cells.get_mut(&port_side) {
            cell.minimum_content_area_size_mut().x = width;
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

        // Set the horizontal port margins
        Self::compute_horizontal_port_margins(node_context, port_side, port_labels_outside);

        if port_labels_outside {
            if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
                let len = port_contexts.len();
                if len > 0 {
                    // First port doesn't need left margin, last port doesn't need right margin
                    port_contexts[0].port_margin.left = 0.0;
                    port_contexts[len - 1].port_margin.right = 0.0;

                    // Space-efficient: first port doesn't need right margin if label not next to it
                    if space_efficient_port_labels && !port_contexts[0].labels_next_to_port {
                        port_contexts[0].port_margin.right = 0.0;
                    }
                }
            }
        }

        if uniform_port_spacing {
            if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
                Self::unify_port_margins(port_contexts);

                if port_labels_outside && !port_contexts.is_empty() {
                    let len = port_contexts.len();
                    port_contexts[0].port_margin.left = 0.0;
                    port_contexts[len - 1].port_margin.right = 0.0;
                }
            }
        }
    }

    fn compute_horizontal_port_margins(
        node_context: &mut NodeContext,
        port_side: PortSide,
        _port_labels_outside: bool,
    ) {
        let is_fixed = PortLabelPlacement::is_fixed(&node_context.port_labels_placement);
        let port_label_spacing_h = node_context.port_label_spacing_horizontal;

        if let Some(port_contexts) = node_context.port_contexts.get_mut(&port_side) {
            for port_context in port_contexts.iter_mut() {
                let label_width = port_context
                    .port_label_cell
                    .as_ref()
                    .map(|c| c.minimum_width())
                    .unwrap_or(0.0);

                if label_width > 0.0 {
                    if port_context.labels_next_to_port {
                        let port_width = port_context.port_size.x;
                        if label_width > port_width {
                            let overhang = (label_width - port_width) / 2.0;
                            port_context.port_margin.left = overhang;
                            port_context.port_margin.right = overhang;
                        }
                    } else {
                        port_context.port_margin.right = port_label_spacing_h + label_width;
                    }
                } else if is_fixed {
                    let labels_bounds = port_context.get_labels_bounds();
                    if labels_bounds.x < 0.0 {
                        port_context.port_margin.left = -labels_bounds.x;
                    }
                    if labels_bounds.x + labels_bounds.width > port_context.port_size.x {
                        port_context.port_margin.right =
                            labels_bounds.x + labels_bounds.width - port_context.port_size.x;
                    }
                }
            }
        }
    }

    fn unify_port_margins(port_contexts: &mut [PortContext]) {
        let mut max_left = 0.0_f64;
        let mut max_right = 0.0_f64;

        for pc in port_contexts.iter() {
            max_left = max_left.max(pc.port_margin.left);
            max_right = max_right.max(pc.port_margin.right);
        }

        for pc in port_contexts.iter_mut() {
            pc.port_margin.left = max_left;
            pc.port_margin.right = max_right;
        }
    }

    fn port_width_plus_port_port_spacing(
        node_context: &NodeContext,
        port_side: PortSide,
    ) -> f64 {
        let mut result = 0.0;

        if let Some(port_contexts) = node_context.port_contexts.get(&port_side) {
            for (i, port_context) in port_contexts.iter().enumerate() {
                result +=
                    port_context.port_margin.left + port_context.port_size.x + port_context.port_margin.right;

                if i + 1 < port_contexts.len() {
                    result += node_context.port_port_spacing;
                }
            }
        }

        result
    }
}
