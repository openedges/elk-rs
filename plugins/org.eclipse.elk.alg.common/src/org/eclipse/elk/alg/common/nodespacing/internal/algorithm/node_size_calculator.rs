use crate::org::eclipse::elk::alg::common::nodespacing::internal::node_context::NodeContext;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortConstraints, PortSide, SizeConstraint, SizeOptions,
};

use super::node_label_and_size_utilities::NodeLabelAndSizeUtilities;

/// Configures the cell system according to the node size constraints and determines
/// the ultimate node size.
///
/// Faithfully ports Java's `NodeSizeCalculator`.
pub struct NodeSizeCalculator;

impl NodeSizeCalculator {
    /// Sets the node's width according to the active node size constraints. Also sets that
    /// width on the cell system and tells it to compute a horizontal layout.
    pub fn set_node_width(node_context: &mut NodeContext) {
        let width;

        if NodeLabelAndSizeUtilities::are_size_constraints_fixed(node_context) {
            // Simply use the node's current width
            width = node_context.node_size.x;
        } else {
            // Ask the cell system how wide it would like to be
            let mut w = if node_context.topdown_layout {
                node_context
                    .node_size
                    .x
                    .max(node_context.node_container.minimum_width())
            } else {
                node_context.node_container.minimum_width()
            };

            // If we include node labels and outside node labels are not to overhang
            if node_context
                .size_constraints
                .contains(&SizeConstraint::NodeLabels)
                && !node_context
                    .size_options
                    .contains(&SizeOptions::OutsideNodeLabelsOverhang)
            {
                if let Some(north_container) =
                    node_context.outside_node_label_containers.get(&PortSide::North)
                {
                    w = w.max(north_container.minimum_width());
                }
                if let Some(south_container) =
                    node_context.outside_node_label_containers.get(&PortSide::South)
                {
                    w = w.max(south_container.minimum_width());
                }
            }

            // The node might have a minimum size set
            if let Some(min_node_size) =
                NodeLabelAndSizeUtilities::get_minimum_node_size(node_context)
            {
                w = w.max(min_node_size.x);
            }

            width = w;
        }

        // Set the node's width
        if node_context.node_size_fixed_graph_size {
            node_context.node_size.x = node_context.node_size.x.max(width);
        } else {
            node_context.node_size.x = width;
        }

        // Set the cell system's width and tell it to compute horizontal coordinates and widths
        {
            let node_cell_rectangle = node_context.node_container.cell_rectangle();
            node_cell_rectangle.x = 0.0;
            node_cell_rectangle.width = width;
        }

        node_context.node_container.layout_children_horizontally();

        // Sync cell rectangles from the container tree to the HashMap copies
        node_context.sync_inside_port_label_cell_rectangles();
    }

    /// Sets the node's height according to the active node size constraints. Also sets that
    /// height on the cell system and tells it to compute a vertical layout.
    pub fn set_node_height(node_context: &mut NodeContext) {
        let height;

        if NodeLabelAndSizeUtilities::are_size_constraints_fixed(node_context) {
            // Simply use the node's current height
            height = node_context.node_size.y;
        } else {
            // Ask the cell system how high it would like to be
            let mut h = if node_context.topdown_layout {
                node_context
                    .node_size
                    .y
                    .max(node_context.node_container.minimum_height())
            } else {
                node_context.node_container.minimum_height()
            };

            // If we include node labels and outside node labels are not to overhang
            if node_context
                .size_constraints
                .contains(&SizeConstraint::NodeLabels)
                && !node_context
                    .size_options
                    .contains(&SizeOptions::OutsideNodeLabelsOverhang)
            {
                if let Some(east_container) =
                    node_context.outside_node_label_containers.get(&PortSide::East)
                {
                    h = h.max(east_container.minimum_height());
                }
                if let Some(west_container) =
                    node_context.outside_node_label_containers.get(&PortSide::West)
                {
                    h = h.max(west_container.minimum_height());
                }
            }

            // The node might have a minimum size set
            if let Some(min_node_size) =
                NodeLabelAndSizeUtilities::get_minimum_node_size(node_context)
            {
                h = h.max(min_node_size.y);
            }

            // If size constraints include ports, but port constraints are FIXED_POS or
            // FIXED_RATIO, we need to manually apply the height required to place eastern
            // and western ports because those heights don't come out of the cell system
            if node_context
                .size_constraints
                .contains(&SizeConstraint::Ports)
            {
                if node_context.port_constraints == PortConstraints::FixedRatio
                    || node_context.port_constraints == PortConstraints::FixedPos
                {
                    if let Some(east_cell) =
                        node_context.inside_port_label_cells.get(&PortSide::East)
                    {
                        h = h.max(east_cell.minimum_height());
                    }
                    if let Some(west_cell) =
                        node_context.inside_port_label_cells.get(&PortSide::West)
                    {
                        h = h.max(west_cell.minimum_height());
                    }
                }
            }

            height = h;
        }

        // Set the node's height
        if node_context.node_size_fixed_graph_size {
            node_context.node_size.y = node_context.node_size.y.max(height);
        } else {
            node_context.node_size.y = height;
        }

        // Set the cell system's height and tell it to compute vertical coordinates and heights
        {
            let node_cell_rectangle = node_context.node_container.cell_rectangle();
            node_cell_rectangle.y = 0.0;
            node_cell_rectangle.height = height;
        }

        node_context.node_container.layout_children_vertically();

        // Sync cell rectangles from the container tree to the HashMap copies
        node_context.sync_inside_port_label_cell_rectangles();
    }
}
